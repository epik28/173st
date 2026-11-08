//! Beta 1.7.3 seeded climate sampling and biome color palette lookup.
//!
//! The scalar sampler follows the native `xv(fd)` 1×1 climate path. Its private
//! octave/simplex implementation preserves the original Java RNG stream and
//! floating-point operation order; it does not provide general world generation.

use crate::java_random::JavaRandom;
use std::fmt;

const TEMPERATURE_SEED_MULTIPLIER: i64 = 9_871;
const RAINFALL_SEED_MULTIPLIER: i64 = 39_811;
const MIXING_SEED_MULTIPLIER: i64 = 543_321;
const TEMPERATURE_SCALE: f64 = 0.025_000_000_372_529_03;
const RAINFALL_SCALE: f64 = 0.050_000_000_745_058_06;
const MIXING_SCALE: f64 = 0.25;
const SIMPLEX_F2: f64 = 0.366_025_403_784_438_6;
const SIMPLEX_G2: f64 = 0.211_324_865_405_187_13;

const GRADIENTS_2D: [[f64; 2]; 12] = [
    [1.0, 1.0],
    [-1.0, 1.0],
    [1.0, -1.0],
    [-1.0, -1.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [1.0, 0.0],
    [-1.0, 0.0],
    [0.0, 1.0],
    [0.0, -1.0],
    [0.0, 1.0],
    [0.0, -1.0],
];

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Climate {
    pub temperature: f64,
    pub rainfall: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ColorError {
    InvalidClimate,
}

impl fmt::Display for ColorError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidClimate => write!(f, "climate values must be finite and in 0..=1"),
        }
    }
}

impl std::error::Error for ColorError {}

/// Reproduces the original scalar `xv.a(x, z, 1, 1)` temperature/rainfall query.
#[derive(Clone, Debug)]
pub struct ClimateSampler {
    temperature: OctaveNoise,
    rainfall: OctaveNoise,
    mixing: OctaveNoise,
}

impl ClimateSampler {
    #[must_use]
    pub fn new(seed: i64) -> Self {
        Self {
            temperature: OctaveNoise::new(seed.wrapping_mul(TEMPERATURE_SEED_MULTIPLIER), 4),
            rainfall: OctaveNoise::new(seed.wrapping_mul(RAINFALL_SEED_MULTIPLIER), 4),
            mixing: OctaveNoise::new(seed.wrapping_mul(MIXING_SEED_MULTIPLIER), 2),
        }
    }

    /// Samples exact integer world coordinates, independent of query order.
    #[must_use]
    pub fn sample(&mut self, x: i32, z: i32) -> Climate {
        let x = f64::from(x);
        let z = f64::from(z);
        let temperature_noise = self.temperature.sample(x, z, TEMPERATURE_SCALE, 0.25);
        let rainfall_noise = self.rainfall.sample(x, z, RAINFALL_SCALE, 1.0 / 3.0);
        let mixing_noise = self.mixing.sample(x, z, MIXING_SCALE, 10.0 / 17.0);

        let wetness = mixing_noise * 1.1 + 0.5;
        let mut temperature = (temperature_noise * 0.15 + 0.7) * (1.0 - 0.01) + wetness * 0.01;
        let mut rainfall = (rainfall_noise * 0.15 + 0.5) * (1.0 - 0.002) + wetness * 0.002;
        temperature = 1.0 - (1.0 - temperature) * (1.0 - temperature);
        if temperature < 0.0 {
            temperature = 0.0;
        }
        if rainfall < 0.0 {
            rainfall = 0.0;
        }
        if temperature > 1.0 {
            temperature = 1.0;
        }
        if rainfall > 1.0 {
            rainfall = 1.0;
        }
        Climate {
            temperature,
            rainfall,
        }
    }
}

/// Computes the native 256×256 climate palette index (row-major ARGB pixels).
pub fn color_index(climate: Climate) -> Result<usize, ColorError> {
    validate_climate(climate)?;
    let humidity = climate.rainfall * climate.temperature;
    let column = ((1.0 - climate.temperature) * 255.0) as usize;
    let row = ((1.0 - humidity) * 255.0) as usize;
    Ok((row << 8) | column)
}

pub fn grass_color(palette: &[u32; 65_536], climate: Climate) -> Result<u32, ColorError> {
    Ok(palette[color_index(climate)?])
}

pub fn foliage_color(
    palette: &[u32; 65_536],
    climate: Climate,
    metadata: u8,
) -> Result<u32, ColorError> {
    validate_climate(climate)?;
    if metadata & 1 == 1 {
        Ok(6_396_257)
    } else if metadata & 2 == 2 {
        Ok(8_431_445)
    } else {
        Ok(palette[color_index(climate)?])
    }
}

fn validate_climate(climate: Climate) -> Result<(), ColorError> {
    if climate.temperature.is_finite()
        && climate.rainfall.is_finite()
        && (0.0..=1.0).contains(&climate.temperature)
        && (0.0..=1.0).contains(&climate.rainfall)
    {
        Ok(())
    } else {
        Err(ColorError::InvalidClimate)
    }
}

#[derive(Clone, Debug)]
struct OctaveNoise {
    octaves: Vec<SimplexNoise>,
}

impl OctaveNoise {
    fn new(seed: i64, count: usize) -> Self {
        let mut random = JavaRandom::new(seed);
        let octaves = (0..count).map(|_| SimplexNoise::new(&mut random)).collect();
        Self { octaves }
    }

    fn sample(&self, x: f64, z: f64, scale: f64, frequency_step: f64) -> f64 {
        let x_scale = scale / 1.5;
        let z_scale = scale / 1.5;
        let mut frequency = 1.0;
        let mut amplitude_divisor = 1.0;
        let mut result = 0.0;
        for octave in &self.octaves {
            let octave_value = octave.sample(x, z, x_scale * frequency, z_scale * frequency);
            result += 0.55 / amplitude_divisor * octave_value;
            frequency *= frequency_step;
            amplitude_divisor *= 0.5;
        }
        result
    }
}

#[derive(Clone, Debug)]
struct SimplexNoise {
    permutation: [u8; 512],
    x_offset: f64,
    z_offset: f64,
}

impl SimplexNoise {
    fn new(random: &mut JavaRandom) -> Self {
        let x_offset = random.next_double() * 256.0;
        let z_offset = random.next_double() * 256.0;
        // The original 2D simplex implementation consumes a third offset even though
        // its planar sampling path only uses x and z offsets.
        let _y_offset = random.next_double() * 256.0;
        let mut permutation = [0_u8; 512];
        for (index, entry) in permutation[..256].iter_mut().enumerate() {
            *entry = index as u8;
        }
        for index in 0..256 {
            let swap = index + random.next_int_bound((256 - index) as i32).unwrap() as usize;
            permutation.swap(index, swap);
            permutation[index + 256] = permutation[index];
        }
        Self {
            permutation,
            x_offset,
            z_offset,
        }
    }

    fn sample(&self, x: f64, z: f64, x_scale: f64, z_scale: f64) -> f64 {
        let x = x * x_scale + self.x_offset;
        let z = z * z_scale + self.z_offset;
        let skew = (x + z) * SIMPLEX_F2;
        let i = simplex_floor(x + skew);
        let j = simplex_floor(z + skew);
        let unskew = f64::from(i + j) * SIMPLEX_G2;
        let x0 = x - (f64::from(i) - unskew);
        let z0 = z - (f64::from(j) - unskew);
        let (i1, j1): (i32, i32) = if x0 > z0 { (1, 0) } else { (0, 1) };
        let x1 = x0 - f64::from(i1) + SIMPLEX_G2;
        let z1 = z0 - f64::from(j1) + SIMPLEX_G2;
        let x2 = x0 - 1.0 + 2.0 * SIMPLEX_G2;
        let z2 = z0 - 1.0 + 2.0 * SIMPLEX_G2;
        let ii = (i & 255) as usize;
        let jj = (j & 255) as usize;
        let gi0 = usize::from(self.permutation[ii + usize::from(self.permutation[jj])]) % 12;
        let gi1 = usize::from(
            self.permutation[ii + i1 as usize + usize::from(self.permutation[jj + j1 as usize])],
        ) % 12;
        let gi2 =
            usize::from(self.permutation[ii + 1 + usize::from(self.permutation[jj + 1])]) % 12;
        let n0 = simplex_corner(GRADIENTS_2D[gi0], x0, z0);
        let n1 = simplex_corner(GRADIENTS_2D[gi1], x1, z1);
        let n2 = simplex_corner(GRADIENTS_2D[gi2], x2, z2);
        70.0 * (n0 + n1 + n2)
    }
}

fn simplex_corner(gradient: [f64; 2], x: f64, z: f64) -> f64 {
    let mut attenuation = 0.5 - x * x - z * z;
    if attenuation < 0.0 {
        0.0
    } else {
        attenuation *= attenuation;
        attenuation *= attenuation;
        attenuation * (gradient[0] * x + gradient[1] * z)
    }
}

/// Matches the pinned simplex bytecode, including its exact-zero `-1` result.
fn simplex_floor(value: f64) -> i32 {
    let truncated = value as i32;
    if value > 0.0 {
        truncated
    } else {
        truncated.wrapping_sub(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simplex_floor_keeps_native_zero_and_negative_boundary_behavior() {
        assert_eq!(simplex_floor(0.0), -1);
        assert_eq!(simplex_floor(-0.0), -1);
        assert_eq!(simplex_floor(f64::from_bits(1_u64 << 63 | 1)), -1);
        assert_eq!(simplex_floor(f64::from_bits(1)), 0);
        assert_eq!(simplex_floor(-1.0), -2);
        assert_eq!(simplex_floor(1.0), 1);
    }

    #[test]
    fn simplex_permutations_are_seeded_doubled_permutations() {
        let noise = SimplexNoise::new(&mut JavaRandom::new(0));
        let mut first_half = noise.permutation[..256].to_vec();
        first_half.sort_unstable();
        assert_eq!(first_half, (0..=255).map(|v| v as u8).collect::<Vec<_>>());
        assert_eq!(&noise.permutation[..256], &noise.permutation[256..]);
        assert_eq!(
            noise.permutation,
            SimplexNoise::new(&mut JavaRandom::new(0)).permutation
        );
        assert_ne!(
            noise.permutation,
            SimplexNoise::new(&mut JavaRandom::new(1)).permutation
        );
    }

    #[test]
    fn scalar_climate_is_deterministic_bounded_and_coordinate_sensitive() {
        for seed in [0, 1, -1, i64::MIN, i64::MAX] {
            let mut sampler = ClimateSampler::new(seed);
            let first = sampler.sample(-12_345, 67_890);
            assert!((0.0..=1.0).contains(&first.temperature));
            assert!((0.0..=1.0).contains(&first.rainfall));
            assert_eq!(first, sampler.sample(-12_345, 67_890));
            assert!((0.0..=1.0).contains(&sampler.sample(30_000_000, -30_000_000).temperature));
        }
        let mut left = ClimateSampler::new(1234);
        let mut right = ClimateSampler::new(1235);
        assert_ne!(left.sample(256, -512), right.sample(256, -512));
    }

    #[test]
    fn native_palette_index_argb_and_leaf_metadata_precedence_are_preserved() {
        let mut palette = vec![0_u32; 65_536].into_boxed_slice();
        for (i, value) in palette.iter_mut().enumerate() {
            *value = i as u32;
        }
        let palette: Box<[u32; 65_536]> = palette.try_into().unwrap();
        let climate = Climate {
            temperature: 0.5,
            rainfall: 0.5,
        };
        let index = (191 << 8) | 127;
        assert_eq!(color_index(climate), Ok(index));
        assert_eq!(grass_color(&palette, climate), Ok(index as u32));
        assert_eq!(foliage_color(&palette, climate, 0), Ok(index as u32));
        assert_eq!(foliage_color(&palette, climate, 1), Ok(6_396_257));
        assert_eq!(foliage_color(&palette, climate, 2), Ok(8_431_445));
        assert_eq!(foliage_color(&palette, climate, 3), Ok(6_396_257));
        assert_eq!(
            color_index(Climate {
                temperature: 0.0,
                rainfall: 0.0
            }),
            Ok(65_535)
        );
        assert_eq!(
            color_index(Climate {
                temperature: 1.0,
                rainfall: 1.0
            }),
            Ok(0)
        );
        for bad in [
            Climate {
                temperature: f64::NAN,
                rainfall: 0.0,
            },
            Climate {
                temperature: -f64::EPSILON,
                rainfall: 0.0,
            },
            Climate {
                temperature: 0.0,
                rainfall: 1.0 + f64::EPSILON,
            },
        ] {
            assert_eq!(color_index(bad), Err(ColorError::InvalidClimate));
            assert_eq!(
                foliage_color(&palette, bad, 1),
                Err(ColorError::InvalidClimate)
            );
        }
    }
}
