//! Decompressor for the LZ-with-range-coder scheme GR2 uses on its sections
//! (the "Oodle0" variant). It combines Lempel-Ziv back-references with the
//! adaptive range coder in [`super::range_coder`]: each block is either a
//! literal byte or a `(length, offset)` copy from already-decoded output.
//!
//! ## Building blocks (standard techniques)
//!
//! - LZ77 sliding-window back-references  a block is either a literal byte or a
//!   `(length, offset)` copy from earlier output:
//!   <https://en.wikipedia.org/wiki/LZ77_and_LZ78>
//! - Context modeling  each token field (literal byte, match length, offset
//!   high/low bits) has its own adaptive model over the shared range coder
//!   ([`super::range_coder`]): <https://en.wikipedia.org/wiki/Context_model>
//! - Section/Oodle0 container layout:
//!   <https://github.com/rdw-archive/RagnarokFileFormats/blob/master/GR2.MD>

use std::cmp::min;

use crate::FormatError;
use crate::gr2::range_coder::{Decoder, Window};

#[derive(Clone, Copy)]
struct Parameter {
    decoded_value_max: u32,
    backref_value_max: u32,
    decoded_count: u32,
    highbit_count: u32,
    sizes_count: [u8; 4],
}

impl Parameter {
    /// Parse a 12-byte parameter block: two packed 32-bit words plus four
    /// symbol counts. Each word splits at bit 9  the low 9 bits and the high
    /// 23 bits are independent fields (see the field assignments below).
    fn read(buf: &[u8]) -> Self {
        debug_assert!(buf.len() >= 12);
        let w0 = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
        let w1 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
        Parameter {
            decoded_value_max: w0 & 0x1ff,
            backref_value_max: w0 >> 9,
            decoded_count: w1 & 0x1ff,
            highbit_count: w1 >> 9,
            sizes_count: [buf[8], buf[9], buf[10], buf[11]],
        }
    }
}

struct Dictionary {
    decoded_size: u32,
    backref_size: u32,
    decoded_value_max: u32,
    backref_value_max: u32,
    lowbit_value_max: u32,
    lowbit_window: Window,
    highbit_window: Window,
    decoded_window: Window,
    size_windows: Vec<Window>,
}

impl Dictionary {
    fn new(p: &Parameter) -> Self {
        let backref_value_max = p.backref_value_max;
        let lowbit_value_max = min(backref_value_max, 4);

        let lowbit_window = Window::new(lowbit_value_max, lowbit_value_max as u16);
        let highbit_window = Window::new(p.highbit_count, p.highbit_count as u16);
        let decoded_window = Window::new(p.decoded_value_max, p.decoded_count as u16);

        let mut size_windows = Vec::with_capacity(4 * 16 + 1);
        for i in 0..4 {
            for _ in 0..16 {
                size_windows.push(Window::new(64, p.sizes_count[3 - i] as u16));
            }
        }
        size_windows.push(Window::new(64, p.sizes_count[0] as u16));

        Dictionary {
            decoded_size: 0,
            backref_size: 0,
            decoded_value_max: p.decoded_value_max,
            backref_value_max,
            lowbit_value_max,
            lowbit_window,
            highbit_window,
            decoded_window,
            size_windows,
        }
    }

    fn decompress_block(
        &mut self,
        decoder: &mut Decoder,
        out: &mut [u8],
        pos: usize,
    ) -> Result<usize, FormatError> {
        let size_idx = self.backref_size as usize;
        self.backref_size = self.size_windows[size_idx]
            .decode_symbol(decoder, |d| d.decode_commit(65)) as u32;

        if self.backref_size > 0 {
            // Length codes 0..=60 map directly to lengths 1..=61; codes 61..=64
            // are escapes for these four larger back-reference lengths.
            const SIZES: [u32; 4] = [128, 192, 256, 512];
            let backref_size = if self.backref_size < 61 {
                self.backref_size + 1
            } else {
                SIZES[(self.backref_size - 61) as usize]
            };
            let backref_range = min(self.backref_value_max, self.decoded_size);

            let lowbit_max = self.lowbit_value_max;
            let low_value = self
                .lowbit_window
                .decode_symbol(decoder, |d| d.decode_commit(lowbit_max));

            let high_total = backref_range / 4 + 1;
            let high_value = self
                .highbit_window
                .decode_symbol(decoder, |d| d.decode_commit(high_total));

            let backref_offset = ((high_value as u32) << 2) + low_value as u32 + 1;

            self.decoded_size += backref_size;

            if pos < backref_offset as usize || pos + backref_size as usize > out.len() {
                return Err(FormatError::DecompressionFailed(format!(
                    "oodle backref out of range: pos={pos} off={backref_offset} size={backref_size}"
                )));
            }
            // Copy one byte at a time: back-references may overlap the output
            // cursor (`src + size > pos`), so each written byte can feed a later
            // read within the same copy. `copy_within` would be incorrect here.
            let src = pos - backref_offset as usize;
            for k in 0..backref_size as usize {
                out[pos + k] = out[src + k];
            }
            Ok(backref_size as usize)
        } else {
            let decoded_max = self.decoded_value_max;
            let byte = self
                .decoded_window
                .decode_symbol(decoder, |d| d.decode_commit(decoded_max));
            if pos >= out.len() {
                return Err(FormatError::DecompressionFailed(
                    "oodle output overflow".into(),
                ));
            }
            out[pos] = (byte & 0xff) as u8;
            self.decoded_size += 1;
            Ok(1)
        }
    }
}

pub(crate) fn decompress(
    compressed: &[u8],
    out: &mut [u8],
    stop0: u32,
    stop1: u32,
) -> Result<(), FormatError> {
    if out.is_empty() {
        return Ok(());
    }
    if compressed.len() < 36 {
        return Err(FormatError::DecompressionFailed("oodle: short header".into()));
    }
    let params = [
        Parameter::read(&compressed[0..12]),
        Parameter::read(&compressed[12..24]),
        Parameter::read(&compressed[24..36]),
    ];

    let mut decoder = Decoder::new(compressed, 36);
    let steps = [stop0 as usize, stop1 as usize, out.len()];
    let mut pos = 0usize;

    for (i, &step) in steps.iter().enumerate() {
        let mut dict = Dictionary::new(&params[i]);
        while pos < step {
            pos += dict.decompress_block(&mut decoder, out, pos)?;
        }
    }
    Ok(())
}
