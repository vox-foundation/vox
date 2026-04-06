//! XorShift RNG + deterministic name hashing (no external deps).

pub(crate) struct Rng(u64);

impl Rng {
    pub(crate) fn new(seed: u64, salt: u64) -> Self {
        let mut s = seed ^ salt;
        if s == 0 {
            s = 0xdeadbeef_cafebabe;
        }
        Self(s)
    }

    pub(crate) fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }

    #[allow(dead_code)]
    pub(crate) fn pick<'a, T>(&mut self, slice: &'a [T]) -> &'a T {
        &slice[self.next() as usize % slice.len()]
    }
}

/// Deterministic string hash (FNV-1a 64-bit) for seeding RNG per tool name.
pub(crate) fn name_hash(s: &str) -> u64 {
    const OFFSET: u64 = 14_695_981_039_346_656_037;
    const PRIME: u64 = 1_099_511_628_211;
    let mut h = OFFSET;
    for &b in s.as_bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(PRIME);
    }
    h
}
