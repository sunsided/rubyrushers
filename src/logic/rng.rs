//! A small seeded random number generator, so the rules need nothing but std.

/// xorshift64*: fast, deterministic for a seed, not cryptographic.
#[derive(Clone, Debug)]
pub struct Rng(u64);

impl Rng {
    /// A generator for `seed`; any seed works, zero included.
    pub fn new(seed: u64) -> Self {
        // splitmix64 step, so neighbouring seeds start far apart and the state is never zero.
        let mut z = seed.wrapping_add(0x9E37_79B9_7F4A_7C15);
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        Self((z ^ (z >> 31)) | 1)
    }

    /// The next 64 random bits.
    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.0 = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A number in `0..n`; `n` must not be zero.
    pub fn below(&mut self, n: u32) -> u32 {
        assert!(n > 0, "Rng::below(0)");
        (((self.next_u64() >> 32) * u64::from(n)) >> 32) as u32
    }

    /// True once in `odds` draws on average; never for `odds == 0`.
    pub fn one_in(&mut self, odds: u32) -> bool {
        odds > 0 && self.below(odds) == 0
    }

    /// Shuffles `items` in place (Fisher-Yates).
    pub fn shuffle<T>(&mut self, items: &mut [T]) {
        for i in (1..items.len()).rev() {
            let j = self.below(i as u32 + 1) as usize;
            items.swap(i, j);
        }
    }
}
