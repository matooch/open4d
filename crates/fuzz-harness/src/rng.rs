//! A tiny seeded generator: reproducible inputs without OS randomness.

/// xorshift64* (Vigna 2016). Never zero-seeded.
#[derive(Clone, Debug)]
pub struct XorShift64(u64);

impl XorShift64 {
    pub fn new(seed: u64) -> XorShift64 {
        XorShift64(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A value in `0..n` (`0` when `n == 0`).
    pub fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            0
        } else {
            (self.next_u64() % n as u64) as usize
        }
    }

    /// `true` with probability `percent` / 100.
    pub fn chance(&mut self, percent: u64) -> bool {
        self.next_u64() % 100 < percent
    }

    /// A float in `[0, 1)`.
    pub fn unit(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    pub fn byte(&mut self) -> u8 {
        (self.next_u64() >> 24) as u8
    }

    pub fn bytes(&mut self, n: usize) -> Vec<u8> {
        (0..n).map(|_| self.byte()).collect()
    }
}

/// Derives a new input from `seeds` (or random bytes when none): flips, truncations,
/// insertions, block duplications, splices with a second seed and interesting-byte stamps.
pub fn mutate(rng: &mut XorShift64, seeds: &[Vec<u8>], max_len: usize) -> Vec<u8> {
    if seeds.is_empty() || rng.chance(5) {
        let n = rng.below(256);
        return rng.bytes(n);
    }
    let mut data = seeds[rng.below(seeds.len())].clone();
    for _ in 0..1 + rng.below(4) {
        match rng.below(8) {
            0 if !data.is_empty() => {
                let i = rng.below(data.len());
                data[i] ^= 1 << rng.below(8);
            }
            1 if !data.is_empty() => {
                let i = rng.below(data.len());
                data[i] = rng.byte();
            }
            2 if !data.is_empty() => {
                let keep = rng.below(data.len());
                data.truncate(keep);
            }
            3 => {
                let i = rng.below(data.len() + 1);
                let n = 1 + rng.below(16);
                let ins = rng.bytes(n);
                data.splice(i..i, ins);
            }
            4 if !data.is_empty() => {
                let a = rng.below(data.len());
                let b = (a + 1 + rng.below(64)).min(data.len());
                let block = data[a..b].to_vec();
                let at = rng.below(data.len() + 1);
                data.splice(at..at, block);
            }
            5 if !data.is_empty() => {
                let other = &seeds[rng.below(seeds.len())];
                if !other.is_empty() {
                    let cut = rng.below(data.len());
                    let from = rng.below(other.len());
                    data.truncate(cut);
                    data.extend_from_slice(&other[from..]);
                }
            }
            6 if !data.is_empty() => {
                const STAMPS: [u8; 8] = [0x00, 0x01, 0x7f, 0x80, 0xff, b'-', b'9', b'"'];
                let i = rng.below(data.len());
                data[i] = STAMPS[rng.below(STAMPS.len())];
            }
            7 if !data.is_empty() => {
                // Delete a run.
                let a = rng.below(data.len());
                let b = (a + 1 + rng.below(32)).min(data.len());
                data.drain(a..b);
            }
            _ => {}
        }
    }
    data.truncate(max_len);
    data
}
