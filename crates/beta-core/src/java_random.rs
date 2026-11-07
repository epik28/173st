//! Java linear congruential generator (LCG) matching `java.util.Random`.

/// Linear congruential generator matching standard `java.util.Random`.
///
/// The recurrence relation is:
/// ```text
/// seed = (seed * 0x5DEECE66D + 0xB) & ((1 << 48) - 1)
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JavaRandom {
    seed: u64,
}

const MULTIPLIER: u64 = 0x5_DEEC_E66D;
const ADDEND: u64 = 0xB;
const MASK: u64 = (1 << 48) - 1;

impl JavaRandom {
    /// Creates a new random number generator using a single 64-bit integer seed.
    ///
    /// Matches `java.util.Random(long seed)` initial scramble:
    /// `(seed ^ multiplier) & mask`.
    #[inline]
    #[must_use]
    pub fn new(seed: i64) -> Self {
        let initial_seed = ((seed as u64) ^ MULTIPLIER) & MASK;
        Self { seed: initial_seed }
    }

    /// Advances the seed state and returns up to 32 pseudorandom bits.
    ///
    /// Matches `java.util.Random.next(int bits)`.
    #[inline]
    fn next(&mut self, bits: u32) -> i32 {
        debug_assert!(bits <= 32);
        self.seed = self.seed.wrapping_mul(MULTIPLIER).wrapping_add(ADDEND) & MASK;
        (self.seed >> (48 - bits)) as i32
    }

    /// Generates the next pseudorandom, uniformly distributed 32-bit integer value.
    ///
    /// Matches `java.util.Random.nextInt()`.
    #[inline]
    pub fn next_int(&mut self) -> i32 {
        self.next(32)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_seed_zero_sequence() {
        let mut rng = JavaRandom::new(0);
        let expected = [
            -1155484576,
            -723955400,
            1033096058,
            -1690734402,
            -1557280266,
        ];
        for &exp in &expected {
            assert_eq!(rng.next_int(), exp);
        }
    }

    #[test]
    fn test_seed_one_sequence() {
        let mut rng = JavaRandom::new(1);
        let expected = [-1155869325, 431529176, 1761283695, 1749940626, 892128508];
        for &exp in &expected {
            assert_eq!(rng.next_int(), exp);
        }
    }

    #[test]
    fn test_seed_negative_one_sequence() {
        let mut rng = JavaRandom::new(-1);
        let expected = [1155099827, 1887904451, 52699159, -1941176418, -1451336087];
        for &exp in &expected {
            assert_eq!(rng.next_int(), exp);
        }
    }

    #[test]
    fn test_seed_forty_two_sequence() {
        let mut rng = JavaRandom::new(42);
        let expected = [-1170105035, 234785527, -1360544799, 205897768, 1325939940];
        for &exp in &expected {
            assert_eq!(rng.next_int(), exp);
        }
    }

    #[test]
    fn test_seed_min_i64_matches_seed_zero() {
        let mut rng_min = JavaRandom::new(i64::MIN);
        let mut rng_zero = JavaRandom::new(0);
        for _ in 0..16 {
            assert_eq!(rng_min.next_int(), rng_zero.next_int());
        }
    }

    #[test]
    fn test_seed_max_i64_matches_seed_negative_one() {
        let mut rng_max = JavaRandom::new(i64::MAX);
        let mut rng_neg_one = JavaRandom::new(-1);
        for _ in 0..16 {
            assert_eq!(rng_max.next_int(), rng_neg_one.next_int());
        }
    }

    #[test]
    fn test_reproducibility() {
        let mut rng1 = JavaRandom::new(123456789);
        let mut rng2 = JavaRandom::new(123456789);
        for _ in 0..100 {
            assert_eq!(rng1.next_int(), rng2.next_int());
        }
    }
}
