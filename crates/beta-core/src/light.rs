//! Beta 1.7.3 saved-light arithmetic and clear-weather sky calculations.
//!
//! This module models scalar light values for ordinary cube samples. It does
//! not propagate light, update weather, or implement AO or special block
//! neighbor sampling.

use crate::math;

const LIGHT_COUNT: usize = 16;
const OVERWORLD_BASE: f32 = 0.05;
const NETHER_BASE: f32 = 0.1;
const PI_F32: f32 = std::f32::consts::PI;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightDimension {
    Overworld,
    Nether,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LightError {
    InvalidLevel,
    InvalidPartialTick,
}

impl std::fmt::Display for LightError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidLevel => formatter.write_str("light level must be in 0..=15"),
            Self::InvalidPartialTick => {
                formatter.write_str("partial tick must be finite and in 0..=1")
            }
        }
    }
}

impl std::error::Error for LightError {}

/// Returns the original provider's scalar brightness lookup for a light level.
pub fn brightness(level: u8, dimension: LightDimension) -> Result<f32, LightError> {
    if level >= LIGHT_COUNT as u8 {
        return Err(LightError::InvalidLevel);
    }
    let base = match dimension {
        LightDimension::Overworld => OVERWORLD_BASE,
        LightDimension::Nether => NETHER_BASE,
    };
    let inverse_level = 1.0_f32 - (f32::from(level) / 15.0_f32);
    let numerator = 1.0_f32 - inverse_level;
    let denominator = inverse_level * 3.0_f32 + 1.0_f32;
    let scaled = (numerator / denominator) * (1.0_f32 - base);
    Ok(scaled + base)
}

/// Combines block and saved-sky nibbles using `lm.c(x,y,z,subtraction)` order.
pub fn combined_light(block: u8, sky: u8, subtraction: u8) -> Result<u8, LightError> {
    if block > 15 || sky > 15 || subtraction > 15 {
        return Err(LightError::InvalidLevel);
    }
    let adjusted_sky = i32::from(sky) - i32::from(subtraction);
    Ok(i32::from(block).max(adjusted_sky) as u8)
}

/// Combines saved nibbles, applies the native minimum, and maps through the
/// dimension's provider brightness table.
pub fn sample_brightness(
    block: u8,
    sky: u8,
    subtraction: u8,
    minimum: u8,
    dimension: LightDimension,
) -> Result<f32, LightError> {
    let combined = combined_light(block, sky, subtraction)?;
    brightness(combined.max(minimum), dimension)
}

/// Returns the provider's celestial angle used by clear-weather lighting.
pub fn celestial_angle(
    time: i64,
    partial: f32,
    dimension: LightDimension,
) -> Result<f32, LightError> {
    validate_partial(partial)?;
    if dimension == LightDimension::Nether {
        return Ok(0.5_f32);
    }

    // xa.a(JF): Java long remainder precedes conversion to f32.
    let day_time = (time % 24_000_i64) as f32;
    let mut phase = day_time + partial;
    phase /= 24_000.0_f32;
    phase -= 0.25_f32;
    if phase < 0.0_f32 {
        phase += 1.0_f32;
    }
    if phase > 1.0_f32 {
        phase -= 1.0_f32;
    }

    // xa.a(JF) uses java.lang.Math.cos(double), unlike fd.a(F)'s MathHelper LUT.
    let original = phase;
    let cosine = ((f64::from(phase) * std::f64::consts::PI).cos() + 1.0_f64) / 2.0_f64;
    phase = 1.0_f32 - cosine as f32;
    phase = original + (phase - original) / 3.0_f32;
    Ok(phase)
}

/// Returns clear-weather sky subtraction for the provider and world time.
#[allow(clippy::manual_clamp)] // Preserve the native sequence of ordered comparisons.
pub fn sky_subtraction(
    time: i64,
    partial: f32,
    dimension: LightDimension,
) -> Result<u8, LightError> {
    let angle = celestial_angle(time, partial, dimension)?;
    let mut twilight = 1.0_f32 - (math::cos(angle * PI_F32 * 2.0_f32) * 2.0_f32 + 0.5_f32);
    if twilight < 0.0_f32 {
        twilight = 0.0_f32;
    }
    if twilight > 1.0_f32 {
        twilight = 1.0_f32;
    }
    let mut sky_brightness = 1.0_f32 - twilight;
    // The native clear-weather path still performs both f32 -> f64 -> f32
    // attenuations; retaining them preserves its operation shape.
    sky_brightness = (f64::from(sky_brightness) * (1.0_f64 - 0.0_f64 * 5.0_f64 / 16.0_f64)) as f32;
    sky_brightness = (f64::from(sky_brightness) * (1.0_f64 - 0.0_f64 * 5.0_f64 / 16.0_f64)) as f32;
    let darkness = 1.0_f32 - sky_brightness;
    Ok((darkness * 11.0_f32) as u8)
}

/// Returns the original static block registry's light-emission level.
#[must_use]
pub fn emission_level(block_id: u8) -> u8 {
    match block_id {
        10 | 11 | 51 | 89 | 91 | 95 => 15,
        39 => 1,
        50 => 14,
        62 => 13,
        74 | 94 => 9,
        76 => 7,
        90 => 11,
        _ => 0,
    }
}

fn validate_partial(partial: f32) -> Result<(), LightError> {
    if partial.is_finite() && (0.0_f32..=1.0_f32).contains(&partial) {
        Ok(())
    } else {
        Err(LightError::InvalidPartialTick)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brightness_tables_match_native_formula_and_dimension_bases() {
        assert_eq!(brightness(0, LightDimension::Overworld), Ok(0.05_f32));
        assert_eq!(brightness(0, LightDimension::Nether), Ok(0.1_f32));
        assert_eq!(brightness(15, LightDimension::Overworld), Ok(1.0_f32));
        assert_eq!(brightness(15, LightDimension::Nether), Ok(1.0_f32));
        assert_eq!(
            brightness(8, LightDimension::Overworld).unwrap().to_bits(),
            0x3e85_b05c
        );
        assert_eq!(
            brightness(8, LightDimension::Nether).unwrap().to_bits(),
            0x3e99_999a
        );
        assert_eq!(
            brightness(16, LightDimension::Overworld),
            Err(LightError::InvalidLevel)
        );
    }

    #[test]
    fn saved_nibbles_apply_subtraction_then_block_and_minimum() {
        assert_eq!(combined_light(5, 12, 4), Ok(8));
        assert_eq!(combined_light(8, 12, 4), Ok(8));
        assert_eq!(combined_light(0, 3, 15), Ok(0));
        assert_eq!(combined_light(16, 0, 0), Err(LightError::InvalidLevel));
        assert_eq!(combined_light(0, 0, 16), Err(LightError::InvalidLevel));
        assert_eq!(
            sample_brightness(1, 4, 0, 7, LightDimension::Overworld),
            brightness(7, LightDimension::Overworld)
        );
        assert_eq!(
            sample_brightness(0, 0, 0, 16, LightDimension::Overworld),
            Err(LightError::InvalidLevel)
        );
    }

    #[test]
    fn celestial_angle_preserves_provider_overrides_and_java_time_remainder() {
        assert_eq!(celestial_angle(0, 0.0, LightDimension::Nether), Ok(0.5));
        assert_eq!(
            celestial_angle(i64::MIN, 1.0, LightDimension::Nether),
            Ok(0.5)
        );
        assert_eq!(
            celestial_angle(0, 1.0, LightDimension::Overworld)
                .unwrap()
                .to_bits(),
            0x3f48_d8fd
        );
        assert_eq!(
            celestial_angle(15_000, 1.0, LightDimension::Overworld)
                .unwrap()
                .to_bits(),
            0x3eb4_b3c5
        );
        assert_eq!(
            celestial_angle(24_000, 0.0, LightDimension::Overworld),
            celestial_angle(0, 0.0, LightDimension::Overworld)
        );
        assert_eq!(
            celestial_angle(-1, 0.0, LightDimension::Overworld),
            celestial_angle(23_999, 0.0, LightDimension::Overworld)
        );
    }

    #[test]
    fn clear_weather_sky_subtraction_matches_native_boundaries_and_cycle() {
        assert_eq!(sky_subtraction(0, 1.0, LightDimension::Overworld), Ok(0));
        assert_eq!(
            sky_subtraction(15_000, 1.0, LightDimension::Overworld),
            Ok(11)
        );
        assert_eq!(sky_subtraction(0, 0.0, LightDimension::Nether), Ok(11));
        for partial in [-0.01, f32::NAN, f32::INFINITY, 1.01] {
            assert_eq!(
                sky_subtraction(0, partial, LightDimension::Overworld),
                Err(LightError::InvalidPartialTick)
            );
        }
        assert_eq!(
            sky_subtraction(0, -0.0, LightDimension::Overworld),
            sky_subtraction(0, 0.0, LightDimension::Overworld)
        );
    }

    #[test]
    fn static_emission_registry_has_only_native_nonzero_levels() {
        let expected = [
            (10, 15),
            (11, 15),
            (39, 1),
            (50, 14),
            (51, 15),
            (62, 13),
            (74, 9),
            (76, 7),
            (89, 15),
            (90, 11),
            (91, 15),
            (94, 9),
            (95, 15),
        ];
        for id in 0..=u8::MAX {
            let level = expected
                .iter()
                .find(|(block, _)| *block == id)
                .map_or(0, |(_, level)| *level);
            assert_eq!(emission_level(id), level, "block id {id}");
        }
    }
}
