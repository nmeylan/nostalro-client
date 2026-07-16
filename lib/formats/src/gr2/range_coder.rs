//! Range decoder with an adaptive frequency model, shared by the Oodle and Bink
//! decoders. [`Decoder`] is the arithmetic (range) coder; [`Window`] is the
//! adaptive model that feeds it symbol frequencies and rescales as it goes.
//!
//! ## Building blocks (standard techniques)
//!
//! - Range coding  narrow a `low`/`high`/`code` interval per symbol, with
//!   byte→nibble→bit renormalization to refill precision:
//!   <https://en.wikipedia.org/wiki/Range_coding>,
//!   <https://en.wikipedia.org/wiki/Arithmetic_coding>
//! - Adaptive frequency model  symbol weights grow as symbols occur and are
//!   periodically halved to age out old statistics ([`Window::rebuild`]):
//!   <https://en.wikipedia.org/wiki/Adaptive_coding>
//! - Cumulative-frequency symbol lookup  [`Window::try_decode`] walks the
//!   weight table to map a decoded value back to its symbol.

/// 4-bit bit-reversal lookup, used to flip nibbles/bytes during the LSB-first
/// renormalization of the range decoder.
pub(crate) const REV4: [u8; 16] = [
    0x0, 0x8, 0x4, 0xc, 0x2, 0xa, 0x6, 0xe, 0x1, 0x9, 0x5, 0xd, 0x3, 0xb, 0x7, 0xf,
];

#[inline]
fn reverse4(n: u32) -> u32 {
    REV4[(n & 0xf) as usize] as u32
}

/// Read 4 bytes little-endian, zero-filling past the end of the stream. The
/// decoder deliberately reads a few bytes beyond the compressed input; a
/// truncated tail then reads as zeros and terminates decoding gracefully
/// instead of erroring.
#[inline]
fn le_u32_or_zero(data: &[u8], off: usize) -> u32 {
    let mut b = [0u8; 4];
    for (i, slot) in b.iter_mut().enumerate() {
        *slot = data.get(off + i).copied().unwrap_or(0);
    }
    u32::from_le_bytes(b)
}

#[inline]
fn reverse8(b: u32) -> u32 {
    (reverse4(b) << 4) | reverse4(b >> 4)
}

pub(crate) struct Reservoir<'a> {
    stream: &'a [u8],
    pos: usize,
    bitbuf: u64,
    bitcount: u32,
}

impl<'a> Reservoir<'a> {
    pub(crate) fn new(data: &'a [u8], start: usize) -> Self {
        Reservoir { stream: data, pos: start, bitbuf: 0, bitcount: 0 }
    }

    pub(crate) fn pull(&mut self, n: u32) -> u32 {
        if self.bitcount < n {
            let dword = le_u32_or_zero(self.stream, self.pos);
            self.pos += 4;
            self.bitbuf |= (dword as u64) << self.bitcount;
            self.bitcount += 32;
        }
        let v = (self.bitbuf & ((1u64 << n) - 1)) as u32;
        self.bitbuf >>= n;
        self.bitcount -= n;
        v
    }
}

pub(crate) struct Decoder<'a> {
    res: Reservoir<'a>,
    low: u32,
    high: u32,
    code: u32,
}

impl<'a> Decoder<'a> {
    pub(crate) fn new(data: &'a [u8], start: usize) -> Self {
        let first = le_u32_or_zero(data, start);
        let mut res = Reservoir::new(data, start + 4);
        res.bitbuf = (first >> 31) as u64;
        res.bitcount = 1;
        Decoder {
            res,
            low: 0,
            high: 0x7fff_ffff,
            code: (first & 0x7fff_ffff).reverse_bits() >> 1,
        }
    }

    fn pull(&mut self, n: u32) -> u32 {
        self.res.pull(n)
    }

    pub(crate) fn decode(&mut self, total: u32) -> u16 {
        let range = (self.high - self.low + 1) as u64;
        let v = ((self.code - self.low + 1) as u64 * total as u64 - 1) / range;
        v as u16
    }

    pub(crate) fn commit(&mut self, total: u32, val: u16, err: u16) -> u16 {
        let range = (self.high - self.low + 1) as u64;
        let total = total as u64;
        self.high = self.low + (range * (val as u64 + err as u64) / total) as u32 - 1;
        self.low += (range * val as u64 / total) as u32;
        self.renormalize();
        val
    }

    pub(crate) fn decode_commit(&mut self, total: u32) -> u16 {
        let v = self.decode(total);
        self.commit(total, v, 1)
    }

    /// Re-expand the interval so the coder keeps ~31 bits of precision, pulling
    /// fresh stream bits into `code` to stay aligned. Standard arithmetic-coding
    /// renormalization; values are 31-bit, so bit 30 (`0x4000_0000`) is the
    /// midpoint ("half") and bit 29 (`0x2000_0000`) is the quarter mark.
    fn renormalize(&mut self) {
        // Settled leading bits: while the top bits of `low` and `high` agree
        // they are decided forever, so shift them out and pull replacements. Do
        // it in the largest matching chunk for speed ; a byte if the top 8 bits
        // match, else a nibble if the top 4 match, else bit-by-bit. `high` fills
        // with 1s and `low` with 0s (widest interval still consistent with the
        // bits removed); `code` takes the same bits, reversed because the stream
        // is least-significant-bit-first.
        if (self.low ^ self.high) & 0x4000_0000 == 0 {
            while (self.low ^ self.high) & 0x7f80_0000 == 0 {
                let byte = self.pull(8);
                self.high = (self.high << 8) | 0xff;
                self.low <<= 8;
                self.code = (self.code << 8) | reverse8(byte);
            }
            if (self.low ^ self.high) & 0x7800_0000 == 0 {
                let nib = self.pull(4);
                self.high = (self.high << 4) | 0xf;
                self.low <<= 4;
                self.code = (self.code << 4) | reverse4(nib);
            }
            while (self.low ^ self.high) & 0x4000_0000 == 0 {
                let bit = self.pull(1);
                self.high = (self.high << 1) | 1;
                self.low <<= 1;
                self.code = (self.code << 1) | bit;
            }
        }
        // Underflow straddle (the "E3" case): the top bits differ, so `low` is in
        // [1/4, 1/2) and `high` in [1/2, 3/4) ; no bit is settled yet but
        // precision is draining. Expand around the midpoint by dropping the
        // quarter bit from all three registers, deferring the decision.
        while self.low & 0x2000_0000 != 0 && self.high & 0x2000_0000 == 0 {
            let bit = self.pull(1);
            self.low = (self.low & 0x1fff_ffff) << 1;
            self.high = (self.high << 1) | 0x4000_0001;
            self.code = ((self.code ^ 0x2000_0000) << 1) | bit;
        }
        // Keep all three registers within 31 bits after the shifts.
        self.low &= 0x7fff_ffff;
        self.high &= 0x7fff_ffff;
        self.code &= 0x7fff_ffff;
    }
}

/// Adaptive frequency model driving the range decoder. It maps decoded
/// positions to symbol `values`, tracks each symbol's `weights` (and their
/// running `total`), and periodically rescales so frequencies track recent
/// input. `step`/`shift`/`step_times_15` parameterize where the most-frequent
/// symbol is kept for fast lookup; `count_cap` bounds the distinct symbol count.
pub(crate) struct Window {
    total: u16,
    num_values: u16,
    step_times_15: u16,
    shift: u8,
    step: u16,
    count_cap: u16,
    values: Vec<u16>,
    weights: Vec<u16>,
}

/// Outcome of decoding one symbol from a [`Window`].
enum Decoded {
    /// Symbol was already present in the window; carries its value.
    Existing(u16),
    /// A previously-unseen symbol was allocated at `slot`; the caller supplies
    /// its raw value, which the window records for next time.
    New { slot: usize },
}

impl Window {
    pub(crate) fn new(_max_value: u32, count_cap: u16) -> Self {
        let cap = ((count_cap as usize) + 5) & !3;
        let mut w = Window {
            total: 0,
            num_values: 0,
            step_times_15: 0,
            shift: 0,
            step: 0,
            count_cap,
            values: vec![0u16; cap],
            weights: vec![0u16; cap],
        };
        w.granularity(count_cap as u32 + 1);
        w.update(0, 3);
        w
    }

    /// Choose the shift/step (power-of-two bucket size) that minimizes the
    /// leftover span for `n` symbols, so the frequency table stays compact.
    fn granularity(&mut self, n: u32) {
        if n < 6 {
            self.step = 0;
            self.shift = 15;
            self.step_times_15 = 0;
            return;
        }
        let mut best_shift = 0u32;
        let mut best_span = u32::MAX;
        let mut shift = 0u32;
        loop {
            let step = 1u32 << shift;
            let mut buckets = (step + n - 1) / step;
            if buckets > 0x10 {
                buckets = 0x10;
            }
            let mut span = n - (buckets - 1) * step;
            if span < step {
                span = step;
            }
            if span < best_span {
                best_span = span;
                best_shift = shift;
            }
            if step > n {
                break;
            }
            shift += 1;
            if shift >= 0x10 {
                break;
            }
        }
        self.step = 1u16 << best_shift;
        self.shift = best_shift as u8;
        self.step_times_15 = (self.step as u32).wrapping_mul(15) as u16;
    }

    fn update(&mut self, index: usize, delta: u16) {
        self.weights[index] = self.weights[index].wrapping_add(delta);
        self.total = self.total.wrapping_add(delta);
    }

    /// Rescale the model once its total weight saturates: halve every weight
    /// (so recent symbols dominate), drop symbols that decay to nothing, and
    /// move the heaviest symbol to the fast-lookup slot chosen by `granularity`.
    fn rebuild(&mut self) {
        self.granularity(self.num_values as u32 + 1);
        self.weights[0] >>= 1;

        let mut max_weight = 0u32;
        let mut max_sym = 0usize;
        if self.num_values >= 1 {
            let mut d = 1usize;
            'outer: loop {
                while self.weights[d] <= 1 {
                    if d as u16 >= self.num_values {
                        self.weights[d] = 0;
                        self.num_values -= 1;
                        break 'outer;
                    }
                    let last = self.num_values as usize;
                    self.weights[d] = self.weights[last];
                    self.weights[last] = 0;
                    self.values[d] = self.values[last];
                    self.num_values -= 1;
                }
                self.weights[d] >>= 1;
                if self.weights[d] as u32 > max_weight {
                    max_weight = self.weights[d] as u32;
                    max_sym = d;
                }
                d += 1;
                if d as u16 > self.num_values {
                    break;
                }
            }
        }

        if max_weight != 0 {
            let step15 = self.step_times_15 as usize;
            let mut target = if (self.num_values as usize) < step15 {
                (self.num_values as usize >> self.shift) << self.shift
            } else {
                step15
            };
            if target == 0 {
                target = 1;
            }
            if max_sym != target {
                self.weights.swap(target, max_sym);
                self.values.swap(target, max_sym);
            }
        }

        if self.num_values != self.count_cap && self.weights[0] == 0 {
            self.weights[0] = 2;
        }

        let mut sum = 0u16;
        for i in 0..=self.num_values as usize {
            sum = sum.wrapping_add(self.weights[i]);
        }
        self.total = sum;
    }

    /// Decode one symbol. If the window has seen it before, returns its value;
    /// otherwise `read_value` reads the raw value from the stream and the window
    /// records it.
    pub(crate) fn decode_symbol(
        &mut self,
        decoder: &mut Decoder,
        read_value: impl FnOnce(&mut Decoder) -> u16,
    ) -> u16 {
        match self.try_decode(decoder) {
            Decoded::Existing(v) => v,
            Decoded::New { slot } => {
                let v = read_value(decoder);
                self.values[slot] = v;
                v
            }
        }
    }

    fn try_decode(&mut self, decoder: &mut Decoder) -> Decoded {
        if self.total >= 0x4000 {
            self.rebuild();
        }

        let total = self.total as u32;
        let freq = decoder.decode(total);

        let mut cum = 0u16;
        let mut index = 0usize;
        loop {
            let next = cum + self.weights[index];
            if freq < next {
                break;
            }
            cum = next;
            index += 1;
        }

        let weight = self.weights[index];
        decoder.commit(total, cum, weight);
        self.weights[index] += 1;
        self.total += 1;

        if index != 0 {
            return Decoded::Existing(self.values[index]);
        }

        self.num_values += 1;
        let new = self.num_values as usize;
        self.values[new] = 0;
        self.update(new, 2);
        if self.num_values == self.count_cap {
            let w0 = self.weights[0];
            self.update(0, w0.wrapping_neg());
        }

        Decoded::New { slot: new }
    }
}
