//! Wavelet texture decoder for GR2 images.
//!
//! ## What a wavelet transform is
//!
//! A wavelet transform represents an image at several resolutions at once.
//! Encoding repeatedly splits the data into two half-size sub-bands:
//!   - a **coarse** band  a blurred, downsampled approximation, and
//!   - a **detail** band  the high-frequency information the coarse band lost.
//!
//! Applied recursively, this leaves one tiny coarse image plus a pyramid of
//! detail bands. Most detail coefficients are near zero, so they compress well;
//! that (plus the entropy coder in [`super::range_coder`]) is where the size
//! savings come from.
//!
//! ## How decoding reverses it
//!
//! [`decode`] runs the inverse ("synthesis"). It starts from the smallest
//! coarse band and, at each stage, **upsamples and adds the detail back** to
//! double the resolution, until the plane reaches full size. Each stage applies
//! a 1-D synthesis filter (the [`Tap`] coefficient tables) first across rows,
//! then down columns. Small bands use a cheaper 2-tap Haar filter instead of the
//! full multi-tap filter. Finally [`color_transform_rgba`] converts the decoded
//! luma/chroma planes to RGBA.
//!
//!
//! ## Building blocks
//!
//! The exact filter coefficients and bitstream are the codec's own, but every
//! stage is a textbook signal-processing / image-compression technique:
//! - Dyadic multiresolution decomposition into coarse + detail sub-bands  a
//!   two-channel filter bank: <https://en.wikipedia.org/wiki/Filter_bank>,
//!   <https://en.wikipedia.org/wiki/Discrete_wavelet_transform>
//! - FIR synthesis filters (the [`Tap`] tables) applied separably, across rows
//!   then down columns: <https://en.wikipedia.org/wiki/Finite_impulse_response>,
//!   <https://en.wikipedia.org/wiki/Separable_filter>
//! - Haar wavelet for small bands (the 2-tap sum/difference path), a lifting
//!   step: <https://en.wikipedia.org/wiki/Haar_wavelet>,
//!   <https://en.wikipedia.org/wiki/Lifting_scheme>
//! - Symmetric/mirror boundary extension at band edges (`mirror_coarse`/
//!   `mirror_detail`)
//! - Spatial predictive coding of band coefficients  predict from neighbours,
//!   code the residual (DPCM):
//!   <https://en.wikipedia.org/wiki/Differential_pulse-code_modulation>
//! - Run-length coding of zero coefficients (`RUN6`/`RUN8_ESCAPE`):
//!   <https://en.wikipedia.org/wiki/Run-length_encoding>
//! - Reversible luma/chroma colour transform in [`color_transform_rgba`], of the
//!   YCoCg / JPEG 2000 RCT family: <https://en.wikipedia.org/wiki/YCoCg>
//! - Q16 fixed-point arithmetic with round-to-nearest ([`round16`]):
//!   <https://en.wikipedia.org/wiki/Fixed-point_arithmetic>
//! - Adaptive range coding of the tokens: [`super::range_coder`]

use crate::FormatError;
use crate::gr2::range_coder::{Decoder, Reservoir, Window};
use crate::gr2::read_u32;

// Run-length escape tables (codec tuning parameters). Short zero/value runs are
// coded directly by their 6- or 8-bit token; the four largest token values are
// escapes that stand for these bigger run lengths instead. `ac_band` indexes
// these when a run token reaches its top four codes (`t1 >= 0x3c`, `t2 >= 0xfc`).
const RUN6_ESCAPE: [u32; 4] = [0x80, 0x100, 0x200, 0x400];
const RUN8_ESCAPE: [u32; 4] = [0x200, 0x400, 0x800, 0xc00];

fn round16(v: i32) -> i16 {
    let bias = (v >> 31) ^ 0x7fff;
    let mut a = bias.wrapping_add(v);
    if a < 0 {
        a = a.wrapping_add(0xffff);
    }
    (a >> 16) as i16
}

/// One tap of a 1-D wavelet synthesis filter: a coefficient applied to a sample
/// `rel` positions away, drawn from either the coarse or the `detail` band.
/// Interleaving both bands in a single tap list keeps the inner loop branch-free.
struct Tap {
    detail: bool,
    rel: isize,
    coeff: i32,
}

/// `tap!(c, ..)` reads the coarse band, `tap!(d, ..)` the detail band.
macro_rules! tap {
    (c, $rel:expr, $co:expr) => {
        Tap {
            detail: false,
            rel: $rel,
            coeff: $co,
        }
    };
    (d, $rel:expr, $co:expr) => {
        Tap {
            detail: true,
            rel: $rel,
            coeff: $co,
        }
    };
}

// Wavelet synthesis (reconstruction) filter coefficients, in Q16 fixed-point
// (divide by 65536 for the real value; `round16` scales the accumulator back
// down). These are the decoder's reconstruction filter: they combine the
// coarse and detail sub-bands into the higher-resolution output. `PASS_A_EVEN`
// produces even output samples, `PASS_A_ODD` the odd ones; the edge variants
// (`PASS_A_LEFT`/`PASS_A_RIGHT`) below use asymmetric stencils where the full
// filter support runs off the band boundary.
const PASS_A_EVEN: [Tap; 7] = [
    tap!(c, -1, -2667),
    tap!(c, 0, 51674),
    tap!(c, 1, -2667),
    tap!(d, -2, -1563),
    tap!(d, -1, 24733),
    tap!(d, 0, 24733),
    tap!(d, 1, -1563),
];
const PASS_A_ODD: [Tap; 9] = [
    tap!(c, -1, -4230),
    tap!(c, 0, 27400),
    tap!(c, 1, 27400),
    tap!(c, 2, -4230),
    tap!(d, -2, -2479),
    tap!(d, -1, 7250),
    tap!(d, 0, -55882),
    tap!(d, 1, 7250),
    tap!(d, 2, -2479),
];

fn plane_count(has_alpha: bool) -> usize {
    if has_alpha { 4 } else { 3 }
}

fn mag_class(x: u32) -> usize {
    (32 - x.leading_zeros()).min(15) as usize
}

/// A rectangular sub-band region inside a plane buffer: `h` rows of `w` samples
/// starting at `base`, consecutive rows `stride` samples apart. Bundling these
/// keeps the band-decode signatures readable and makes mis-ordering impossible.
struct Band<'a> {
    plane: &'a mut [i16],
    base: usize,
    stride: usize,
    w: usize,
    h: usize,
}

impl Band<'_> {
    fn fill(&mut self, v: i16) {
        for r in 0..self.h {
            let s = self.base + r * self.stride;
            self.plane[s..s + self.w].fill(v);
        }
    }
}

struct BandModels {
    levels: Vec<Window>,
    run6: Window,
    run8: Window,
}

fn setup_models(token: u32, classes: usize) -> BandModels {
    BandModels {
        levels: (0..classes)
            .map(|_| Window::new(token, (token + 1) as u16))
            .collect(),
        run6: Window::new(0x3f, 0x40),
        run8: Window::new(0xff, 0x100),
    }
}

/// Decode the DC (coarse) sub-band: the single smallest, blurriest
/// approximation image at the top of the wavelet pyramid. Its samples vary
/// smoothly and are strongly correlated with their neighbours, so they are
/// coded predictively (DPCM): the first sample is stored raw, each later sample
/// is predicted from already-decoded neighbours (the left sample along the top
/// row, then the average of the left and above samples), and only the small
/// residual `delta` (magnitude + sign) is decoded. A leading 1-bit flags the
/// whole band as a single constant value, which is filled directly.
fn dc_band(dec: &mut Decoder, rd: &mut Reservoir, mut band: Band<'_>) {
    if rd.pull(1) != 0 {
        let v = rd.pull(16) as i16;
        band.fill(v);
        return;
    }
    let Band {
        plane,
        base,
        stride,
        w,
        h,
    } = band;
    let max = rd.pull(16);
    let total = max + 1;
    let mut model = Window::new(max, total as u16);

    let delta = |dec: &mut Decoder, rd: &mut Reservoir, model: &mut Window| -> i32 {
        let mut v = model.decode_symbol(dec, |d| d.decode_commit(total)) as i32;
        if v != 0 && rd.pull(1) != 0 {
            v = -v;
        }
        v
    };

    let mut dst = base;
    let mut left = rd.pull(16) as i32;
    plane[dst] = left as i16;
    dst += 1;
    for _ in 1..w {
        left += delta(dec, rd, &mut model);
        plane[dst] = left as i16;
        dst += 1;
    }
    let row_gap = stride - w;
    for _ in 1..h {
        dst += row_gap;
        let mut above = dst - stride;
        let mut left = plane[above] as i32 + delta(dec, rd, &mut model);
        plane[dst] = left as i16;
        dst += 1;
        above += 1;
        for _ in 1..w {
            let s = plane[above] as i32 + left;
            let pred = (s + (s < 0) as i32) >> 1;
            left = pred + delta(dec, rd, &mut model);
            plane[dst] = left as i16;
            dst += 1;
            above += 1;
        }
    }
}

/// Decode one AC (detail) sub-band: the high-frequency information added back at
/// a pyramid level (three per level; horizontal, vertical, diagonal detail).
/// These are residuals, so most coefficients are zero with occasional spikes,
/// which drives a different strategy from [`dc_band`]:
/// - Long stretches of zeros are run-length coded. `run6`/`run8` decode a run of
///   coded values (`r1`) followed by a run of literal zeros (`r2`); the escape
///   tables [`RUN6_ESCAPE`]/[`RUN8_ESCAPE`] extend the largest tokens.
/// - Each nonzero coefficient is context-modeled: a magnitude class is predicted
///   from the surrounding already-decoded neighbours ([`mag_class`] of their
///   average) and the level is decoded with the frequency model for that class,
///   then scaled by the band `scale` and given a sign bit.
///
/// A leading 1-bit again flags a constant band, filled directly.
fn ac_band(dec: &mut Decoder, rd: &mut Reservoir, mut band: Band<'_>) -> Result<(), FormatError> {
    let scale = rd.pull(16) as i32;
    if rd.pull(1) != 0 {
        let v = (rd.pull(16) as i32).wrapping_mul(scale) as i16;
        band.fill(v);
        return Ok(());
    }
    let Band {
        plane,
        base,
        stride,
        w,
        h,
    } = band;
    let token = rd.pull(16);
    let esc_total = token + 1;
    let classes = mag_class(token.wrapping_mul(scale as u32)) + 1;
    let mut m = setup_models(token, classes);

    let mut v = dec.decode_commit(esc_total) as i32;
    if v != 0 {
        if rd.pull(1) != 0 {
            v = -v;
        }
        v = v.wrapping_mul(scale);
    }
    plane[base] = v as i16;
    let mut dst = base + 1;
    let mut above = base;
    let mut above_val = v;
    let mut aa = v;
    let mut left = v;
    let mut rows = h;
    let row_gap = stride - w;
    let mut cols = if w == 1 { 0 } else { w - 1 };
    let mut r1 = 0u32;
    let mut r2 = 0u32;
    let mut budget = w * h * 4 + 4096;

    loop {
        if r1 == 0 {
            if r2 == 0 {
                budget = budget
                    .checked_sub(1)
                    .ok_or_else(|| FormatError::DecompressionFailed("bink: band overrun".into()))?;
                let t1 = m.run6.decode_symbol(dec, |_| rd.pull(6) as u16) as u32;
                r1 = if t1 >= 0x3c {
                    RUN6_ESCAPE[(t1 - 0x3c) as usize]
                } else {
                    t1
                };
                let t2 = m.run8.decode_symbol(dec, |_| rd.pull(8) as u16) as u32;
                r2 = if t2 >= 0xfc {
                    RUN8_ESCAPE[(t2 - 0xfc) as usize] + 2
                } else if t2 != 0 {
                    t2 + 2
                } else {
                    0
                };
                continue;
            }
            if (r2 as usize) < cols {
                cols -= r2 as usize;
                for _ in 0..r2 {
                    plane[dst] = 0;
                    dst += 1;
                }
                above += r2 as usize;
                aa = plane[above - 2] as i32;
                above_val = plane[above - 1] as i32;
                left = 0;
                r2 = 0;
            } else {
                r2 -= cols as u32;
                for _ in 0..cols {
                    plane[dst] = 0;
                    dst += 1;
                }
                rows -= 1;
                if rows == 0 {
                    return Ok(());
                }
                dst += row_gap;
                above = dst - stride;
                above_val = plane[above] as i32;
                above += 1;
                aa = above_val;
                left = above_val;
                cols = w;
            }
            continue;
        }
        if cols == 0 {
            rows -= 1;
            if rows == 0 {
                return Ok(());
            }
            dst += row_gap;
            above = dst - stride;
            above_val = plane[above] as i32;
            above += 1;
            aa = above_val;
            left = above_val;
            cols = w;
            continue;
        }
        let last_col = cols == 1;
        let pred = if last_col {
            (left.wrapping_mul(2).unsigned_abs())
                .wrapping_add(aa.unsigned_abs())
                .wrapping_add(above_val.unsigned_abs())
                >> 2
        } else {
            (plane[above] as i32)
                .unsigned_abs()
                .wrapping_add(aa.unsigned_abs())
                .wrapping_add(above_val.unsigned_abs())
                .wrapping_add(left.unsigned_abs())
                >> 2
        };
        let cls = mag_class(pred);
        let mut lvl = m.levels[cls].decode_symbol(dec, |d| d.decode_commit(esc_total)) as i32;
        if lvl != 0 {
            if rd.pull(1) != 0 {
                lvl = -lvl;
            }
            lvl = lvl.wrapping_mul(scale);
        }
        plane[dst] = lvl as i16;
        dst += 1;
        r1 -= 1;
        if last_col {
            rows -= 1;
            if rows == 0 {
                return Ok(());
            }
            dst += row_gap;
            above = dst - stride;
            above_val = plane[above] as i32;
            above += 1;
            aa = above_val;
            left = above_val;
            cols = w;
        } else {
            aa = above_val;
            above_val = plane[above] as i32;
            above += 1;
            left = lvl;
            cols -= 1;
        }
    }
}

/// Decode one plane's stream and return the number of bytes it consumed.
///
/// The stream layout (the read order is the format; there is no separate table
/// describing it) is, in sequence:
///
/// ```text
/// u32 len_a                     ; byte length of the range-coded stream
/// u32 len_b                     ; byte length of the raw-bit stream
/// <range-coded stream>          ; len_a bytes, starting at offset 8
/// <raw-bit stream>              ; len_b bytes, starting at offset 8 + len_a
/// ```
///
/// Two streams are read in parallel and interleaved token by token: the range
/// `Decoder` (offset 8) supplies adaptively-modeled symbols, while the bit
/// `Reservoir` (offset 8 + len_a) supplies fixed-width raw fields (scales,
/// flags, token maxima, sign bits). The sub-bands are then decoded in
/// coarse-to-fine order:
///
/// ```text
/// dc_band                       ; 1 DC band at 1/16 resolution
/// for sh in 4, 3, 2, 1:         ; each pyramid level, coarse first
///     ac_band x3                ; horizontal, vertical, diagonal detail
/// row_flags                     ; threshold, then one range symbol per row
/// ```
///
/// The per-row flags select, for the final synthesis stage, which rows carry
/// detail versus repeat their coarse value.
fn decode_plane(
    src: &[u8],
    plane: &mut [i16],
    w: usize,
    h: usize,
    row_flags: &mut [u8],
) -> Result<usize, FormatError> {
    let len_a = read_u32(src, 0)? as usize;
    let len_b = read_u32(src, 4)? as usize;
    let end = 8 + len_a + len_b;
    if src.len() < end {
        return Err(FormatError::UnexpectedEof);
    }
    let mut dec = Decoder::new(src, 8);
    let mut rd = Reservoir::new(src, 8 + len_a);

    dc_band(
        &mut dec,
        &mut rd,
        Band {
            plane: &mut *plane,
            base: 0,
            stride: w << 4,
            w: w >> 4,
            h: h >> 4,
        },
    );
    for sh in (1..=4).rev() {
        let stride = w << sh;
        let (bw, bh) = (w >> sh, h >> sh);
        ac_band(
            &mut dec,
            &mut rd,
            Band {
                plane: &mut *plane,
                base: w >> sh,
                stride,
                w: bw,
                h: bh,
            },
        )?;
        ac_band(
            &mut dec,
            &mut rd,
            Band {
                plane: &mut *plane,
                base: w << (sh - 1),
                stride,
                w: bw,
                h: bh,
            },
        )?;
        ac_band(
            &mut dec,
            &mut rd,
            Band {
                plane: &mut *plane,
                base: (w >> sh) + (w << (sh - 1)),
                stride,
                w: bw,
                h: bh,
            },
        )?;
    }

    let count = h as u32;
    let threshold = dec.decode_commit(count + 1);
    for f in row_flags.iter_mut().take(h) {
        let t = dec.decode(count);
        if t < threshold {
            *f = 0;
            dec.commit(count, 0, threshold);
        } else {
            *f = 1;
            dec.commit(count, threshold, count as u16 - threshold);
        }
    }
    Ok(end)
}

pub(crate) struct DecodedPlanes {
    pub(crate) planes: Vec<Vec<i16>>,
    pub(crate) row_flags: Vec<Vec<u8>>,
    pub(crate) w: usize,
    pub(crate) h: usize,
}

/// Decode every colour plane of a texture. The payload is a 4-byte prefix
/// followed by the plane streams back-to-back (`plane_count` of them: 3, or 4
/// with alpha), each parsed in turn by [`decode_plane`], which reports how many
/// bytes it consumed so the next plane starts at the right offset.
pub(crate) fn decode_planes(
    pixels: &[u8],
    width: usize,
    height: usize,
    has_alpha: bool,
) -> Result<DecodedPlanes, FormatError> {
    if width * height <= 0x100 {
        return Err(FormatError::DecompressionFailed(
            "bink: tiny-texture path not implemented".into(),
        ));
    }
    let w = (width + 15) & !15;
    let h = (height + 15) & !15;
    let n = plane_count(has_alpha);
    let mut planes = Vec::with_capacity(n);
    let mut row_flags = Vec::with_capacity(n);
    let mut off = 4usize;
    for _ in 0..n {
        let mut plane = vec![0i16; w * h];
        let mut flags = vec![0u8; h];
        let consumed = decode_plane(&pixels[off..], &mut plane, w, h, &mut flags)?;
        off += consumed;
        planes.push(plane);
        row_flags.push(flags);
    }
    Ok(DecodedPlanes {
        planes,
        row_flags,
        w,
        h,
    })
}

fn color_transform_rgba(planes: &[Vec<i16>], width: usize, height: usize) -> Vec<u8> {
    let has_alpha = planes.len() >= 4;
    let mut out = vec![0u8; width * height * 4];
    for (i, px) in out.chunks_exact_mut(4).enumerate() {
        let p0 = planes[0][i] as i32;
        let p1 = planes[1][i] as i32;
        let p2 = planes[2][i] as i32;
        let s = p1 + p2;
        let delta = if s < 0 { s + 3 } else { s } >> 2;
        let y = p0 - delta;
        px[0] = (p1 + y).clamp(0, 255) as u8;
        px[1] = y.clamp(0, 255) as u8;
        px[2] = (p2 + y).clamp(0, 255) as u8;
        px[3] = if has_alpha {
            (planes[3][i] as i32).clamp(0, 255) as u8
        } else {
            255
        };
    }
    out
}

const PASS_A_LEFT: [&[Tap]; 4] = [
    &[
        tap!(c, 0, 51674),
        tap!(c, 1, -5334),
        tap!(d, 0, 49466),
        tap!(d, 1, -3126),
    ],
    &[
        tap!(c, 0, 27400),
        tap!(c, 1, 23170),
        tap!(c, 2, -4230),
        tap!(d, 0, -48632),
        tap!(d, 1, 4771),
        tap!(d, 2, -2479),
    ],
    &[
        tap!(c, 0, -2667),
        tap!(c, 1, 51674),
        tap!(c, 2, -2667),
        tap!(d, 0, 23170),
        tap!(d, 1, 24733),
        tap!(d, 2, -1563),
    ],
    &[
        tap!(c, 0, -4230),
        tap!(c, 1, 27400),
        tap!(c, 2, 27400),
        tap!(c, 3, -4230),
        tap!(d, 0, 4771),
        tap!(d, 1, -55882),
        tap!(d, 2, 7250),
        tap!(d, 3, -2479),
    ],
];
const PASS_A_RIGHT: [&[Tap]; 4] = [
    &[
        tap!(c, -3, -2667),
        tap!(c, -2, 51674),
        tap!(c, -1, -2667),
        tap!(d, -4, -1563),
        tap!(d, -3, 24733),
        tap!(d, -2, 24733),
        tap!(d, -1, -1563),
    ],
    &[
        tap!(c, -3, -4230),
        tap!(c, -2, 27400),
        tap!(c, -1, 23170),
        tap!(d, -4, -2479),
        tap!(d, -3, 7250),
        tap!(d, -2, -58361),
        tap!(d, -1, 7250),
    ],
    &[
        tap!(c, -2, -2667),
        tap!(c, -1, 49007),
        tap!(d, -3, -1563),
        tap!(d, -2, 23170),
        tap!(d, -1, 24733),
    ],
    &[
        tap!(c, -2, -8460),
        tap!(c, -1, 54800),
        tap!(d, -3, -4958),
        tap!(d, -2, 14500),
        tap!(d, -1, -55882),
    ],
];

fn pass_a_row(coarse: &[i16], detail: &[i16], out: &mut [i16]) {
    let w = out.len();
    let half = w / 2;
    let band = |t: &Tap, k: isize| -> i32 {
        let arr = if t.detail { detail } else { coarse };
        let i = k + t.rel;
        if i >= 0 && (i as usize) < half {
            arr[i as usize] as i32
        } else {
            0
        }
    };
    for o in 0..w {
        let mut acc = 0i32;
        if o < 4 {
            for t in PASS_A_LEFT[o] {
                let arr = if t.detail { detail } else { coarse };
                acc = acc.wrapping_add(t.coeff.wrapping_mul(arr[t.rel as usize] as i32));
            }
        } else if o >= w - 4 {
            for t in PASS_A_RIGHT[o - (w - 4)] {
                let arr = if t.detail { detail } else { coarse };
                let idx = (half as isize + t.rel) as usize;
                acc = acc.wrapping_add(t.coeff.wrapping_mul(arr[idx] as i32));
            }
        } else {
            let k = (o / 2) as isize;
            let stencil: &[Tap] = if o % 2 == 0 {
                &PASS_A_EVEN
            } else {
                &PASS_A_ODD
            };
            for t in stencil {
                acc = acc.wrapping_add(t.coeff.wrapping_mul(band(t, k)));
            }
        }
        out[o] = round16(acc);
    }
}

fn mirror_coarse(i: isize, n: usize) -> usize {
    if i < 0 {
        (-i) as usize
    } else if i >= n as isize {
        2 * n - 1 - i as usize
    } else {
        i as usize
    }
}

fn mirror_detail(i: isize, n: usize) -> usize {
    if i < 0 {
        (-i - 1) as usize
    } else if i >= n as isize {
        2 * n - 2 - i as usize
    } else {
        i as usize
    }
}

fn pass_b_column(scratch: &[i16], ow: usize, j: usize, n: usize, plane: &mut [i16], pitch: usize) {
    let c = |i: isize| scratch[2 * mirror_coarse(i, n) * ow + j] as i32;
    let d = |i: isize| scratch[(2 * mirror_detail(i, n) + 1) * ow + j] as i32;
    for k in 0..n as isize {
        let even = c(k)
            .wrapping_mul(51674)
            .wrapping_sub((c(k - 1) + c(k + 1)).wrapping_mul(2667))
            .wrapping_add((d(k - 1) + d(k)).wrapping_mul(24733))
            .wrapping_sub((d(k - 2) + d(k + 1)).wrapping_mul(1563));
        let odd = (c(k) + c(k + 1))
            .wrapping_mul(27400)
            .wrapping_sub((c(k - 1) + c(k + 2)).wrapping_mul(4230))
            .wrapping_add((d(k - 1) + d(k + 1)).wrapping_mul(7250))
            .wrapping_sub((d(k - 2) + d(k + 2)).wrapping_mul(2479))
            .wrapping_sub(d(k).wrapping_mul(55882));
        let o = 2 * k as usize * pitch + j;
        plane[o] = round16(even);
        plane[o + pitch] = round16(odd);
    }
}

fn haar_round(v: i32) -> i16 {
    let mut t = v.wrapping_add((v >> 31) ^ 1);
    t -= t >> 31;
    (t >> 1) as i16
}

fn pass_a_row_haar(coarse: &[i16], detail: &[i16], out: &mut [i16], flag0: bool) {
    for (k, c) in coarse.iter().enumerate() {
        let c = *c as i32;
        if flag0 {
            out[2 * k] = c as i16;
            out[2 * k + 1] = c as i16;
        } else {
            let d = detail[k] as i32;
            out[2 * k] = haar_round(2 * c + d);
            out[2 * k + 1] = haar_round(2 * c - d);
        }
    }
}

fn pass_b_column_haar(
    scratch: &[i16],
    ow: usize,
    j: usize,
    n: usize,
    plane: &mut [i16],
    pitch: usize,
) {
    for k in 0..n {
        let c = scratch[2 * k * ow + j] as i32;
        let d = scratch[(2 * k + 1) * ow + j] as i32;
        let o = 2 * k * pitch + j;
        plane[o] = haar_round(2 * c + d);
        plane[o + pitch] = haar_round(2 * c - d);
    }
}

/// One synthesis stage: reconstruct an `ow`×`oh` block from its coarse and
/// detail sub-bands. Pass A filters across rows into a scratch buffer, pass B
/// filters down columns back into the plane. Bands below the filter's support
/// (`ow < 12` / `oh < 10`) fall back to the 2-tap Haar transform.
pub(crate) fn synth_stage(
    plane: &mut [i16],
    w: usize,
    h: usize,
    ow: usize,
    oh: usize,
    flags: Option<&[u8]>,
) {
    let pitch = (h / oh) * w;
    let half = ow / 2;
    let a_full = ow >= 12;
    let b_full = oh >= 10;
    let zeros = vec![0i16; half];
    let mut scratch = vec![0i16; ow * oh];
    for (r, out) in scratch.chunks_exact_mut(ow).enumerate() {
        let src = &plane[r * pitch..r * pitch + ow];
        let flag0 = flags.is_some_and(|f| f[r] == 0);
        if a_full {
            let detail = if flag0 { &zeros } else { &src[half..] };
            pass_a_row(&src[..half], detail, out);
        } else {
            pass_a_row_haar(&src[..half], &src[half..], out, flag0);
        }
    }
    for j in 0..ow {
        if b_full {
            pass_b_column(&scratch, ow, j, oh / 2, plane, pitch);
        } else {
            pass_b_column_haar(&scratch, ow, j, oh / 2, plane, pitch);
        }
    }
}

/// Reconstruct a full-resolution plane from its wavelet pyramid. Starting at
/// 1/8 resolution, each stage doubles the output size by adding one level of
/// detail back to the coarse band, until it reaches `w`×`h`. Per-row flags apply
/// only to the final (full-resolution) stage.
fn synthesize_plane(plane: &mut [i16], w: usize, h: usize, flags: &[u8]) {
    let (mut ow, mut oh) = (w / 8, h / 8);
    while ow <= w {
        let f = if ow == w { Some(flags) } else { None };
        synth_stage(plane, w, h, ow, oh, f);
        ow *= 2;
        oh *= 2;
    }
}

pub fn decode(
    pixels: &[u8],
    width: i32,
    height: i32,
    has_alpha: bool,
) -> Result<Vec<u8>, FormatError> {
    let (width, height) = (width as usize, height as usize);
    // Tiny textures (≤ 0x100 pixels) skip the wavelet codec entirely: the
    // stored pixels are raw RGBA and the reference decoder only runs its
    // output-layout conversion (identity for RGBA8888). Verified byte-exact
    // against it on the guild flag's 16x16 emblem texture.
    if width * height <= 0x100 {
        let expected = width * height * 4;
        return pixels
            .get(..expected)
            .map(<[u8]>::to_vec)
            .ok_or(FormatError::UnexpectedEof);
    }
    let mut decoded = decode_planes(pixels, width, height, has_alpha)?;
    let (w, h) = (decoded.w, decoded.h);
    for (plane, flags) in decoded.planes.iter_mut().zip(&decoded.row_flags) {
        synthesize_plane(plane, w, h, flags);
    }
    let rgba = color_transform_rgba(&decoded.planes, w, h);
    if w == width && h == height {
        return Ok(rgba);
    }
    let mut out = Vec::with_capacity(width * height * 4);
    for row in 0..height {
        let s = row * w * 4;
        out.extend_from_slice(&rgba[s..s + width * 4]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn color_transform_matches_reference_pixel() {
        let planes = vec![vec![159i16], vec![47i16], vec![-111i16], vec![255i16]];
        let rgba = color_transform_rgba(&planes, 1, 1);
        assert_eq!(rgba, vec![222, 175, 64, 255]);
    }

    #[test]
    fn color_transform_clamps_and_defaults_alpha() {
        let planes = vec![vec![300i16], vec![0i16], vec![0i16]];
        let rgba = color_transform_rgba(&planes, 1, 1);
        assert_eq!(rgba, vec![255, 255, 255, 255]);
    }
}
