//! Java linear congruential generator (LCG) matching `java.util.Random`.

use std::error::Error;
use std::fmt;

/// Error returned when an invalid argument is provided to a random generation method.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RandomError {
    /// The specified upper bound was less than or equal to zero.
    InvalidBound,
}

impl fmt::Display for RandomError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBound => write!(f, "bound must be positive"),
        }
    }
}

impl Error for RandomError {}

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
        let mut rng = Self { seed: 0 };
        rng.set_seed(seed);
        rng
    }

    /// Sets the seed of this random number generator using a single 64-bit integer seed.
    ///
    /// Matches `java.util.Random.setSeed(long seed)`.
    #[inline]
    pub fn set_seed(&mut self, seed: i64) {
        self.seed = ((seed as u64) ^ MULTIPLIER) & MASK;
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

    /// Generates a pseudorandom, uniformly distributed value between 0 (inclusive)
    /// and the specified value (exclusive).
    ///
    /// Returns `Err(RandomError::InvalidBound)` if `bound <= 0`, leaving the generator state unchanged.
    /// Matches `java.util.Random.nextInt(int bound)`.
    pub fn next_int_bound(&mut self, bound: i32) -> Result<i32, RandomError> {
        if bound <= 0 {
            return Err(RandomError::InvalidBound);
        }

        let mut r = self.next(31);
        let m = bound - 1;
        if (bound & m) == 0 {
            // Power of 2 fast path
            r = (((bound as i64).wrapping_mul(r as i64)) >> 31) as i32;
        } else {
            // Rejection sampling
            let mut u = r;
            r = u % bound;
            while u.wrapping_sub(r).wrapping_add(m) < 0 {
                u = self.next(31);
                r = u % bound;
            }
        }
        Ok(r)
    }

    /// Generates the next pseudorandom, uniformly distributed 64-bit integer value.
    ///
    /// Matches `java.util.Random.nextLong()`, including signed intermediate additions.
    #[inline]
    pub fn next_long(&mut self) -> i64 {
        let hi = self.next(32) as i64;
        let lo = self.next(32) as i64;
        (hi << 32).wrapping_add(lo)
    }

    /// Generates the next pseudorandom, uniformly distributed boolean value.
    ///
    /// Matches `java.util.Random.nextBoolean()`.
    #[inline]
    pub fn next_boolean(&mut self) -> bool {
        self.next(1) != 0
    }

    /// Generates the next pseudorandom, uniformly distributed floating point value
    /// between `0.0` (inclusive) and `1.0` (exclusive).
    ///
    /// Matches `java.util.Random.nextFloat()`.
    #[inline]
    pub fn next_float(&mut self) -> f32 {
        (self.next(24) as f32) / ((1u32 << 24) as f32)
    }

    /// Generates the next pseudorandom, uniformly distributed double value
    /// between `0.0` (inclusive) and `1.0` (exclusive).
    ///
    /// Matches `java.util.Random.nextDouble()`.
    #[inline]
    pub fn next_double(&mut self) -> f64 {
        let hi = (self.next(26) as i64) << 27;
        let lo = self.next(27) as i64;
        ((hi + lo) as f64) / ((1u64 << 53) as f64)
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

    #[test]
    fn test_invalid_bound_does_not_mutate_state() {
        let mut rng = JavaRandom::new(0);
        let initial_val = -1155484576;
        assert_eq!(rng.next_int(), initial_val);

        // State before failed bound calls
        let state_before = rng.seed;
        assert_eq!(rng.next_int_bound(0), Err(RandomError::InvalidBound));
        assert_eq!(rng.seed, state_before);

        assert_eq!(rng.next_int_bound(-1), Err(RandomError::InvalidBound));
        assert_eq!(rng.seed, state_before);

        assert_eq!(rng.next_int_bound(i32::MIN), Err(RandomError::InvalidBound));
        assert_eq!(rng.seed, state_before);

        // Next call produces the exact next number as if no invalid bounds were called
        assert_eq!(rng.next_int(), -723955400);
    }

    #[test]
    fn test_reseed_matches_new() {
        let mut rng1 = JavaRandom::new(0);
        let mut rng2 = JavaRandom::new(999);
        rng2.set_seed(0);

        for _ in 0..50 {
            assert_eq!(rng1.next_int(), rng2.next_int());
        }
    }

    #[test]
    fn test_extended_methods_shared_state_sequence() {
        // Verified against standard Java 17 java.util.Random(0L)
        let mut rng = JavaRandom::new(0);
        assert_eq!(rng.next_int(), -1155484576);
        assert_eq!(rng.next_int_bound(16), Ok(13));
        assert_eq!(rng.next_int_bound(7), Ok(4));
        assert_eq!(rng.next_long(), -7261648964369397258);
        assert!(!rng.next_boolean());
        assert_eq!(rng.next_float().to_bits(), 0x3f0ce970);
        assert_eq!(rng.next_double().to_bits(), 0x3fbdf425263e2e88);

        rng.set_seed(42);
        assert_eq!(rng.next_int(), -1170105035);
        assert_eq!(rng.next_int_bound(1073741825), Ok(117392763));
    }

    #[test]
    fn test_rejection_sampling_bound_1073741825() {
        let mut rng = JavaRandom::new(0);
        // Execute 2048 bounded calls on bound=1073741825 to verify rejection sampling doesn't panic
        for _ in 0..2048 {
            let val = rng.next_int_bound(1073741825).unwrap();
            assert!((0..1073741825).contains(&val));
        }
    }
}
