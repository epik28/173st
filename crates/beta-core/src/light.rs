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

/// Returns the original static block registry's raw light-opacity value.
///
/// The native arrays also contain zero for unregistered IDs. This value does
/// not establish that such a block is supported or may be placed by the viewer.
/// Propagation clamps attenuation separately, preserving transparent columns.
#[must_use]
pub fn opacity_level(block_id: u8) -> u8 {
    match block_id {
        18 | 30 => 1,
        8 | 9 | 79 => 3,
        1..=5
        | 7
        | 10..=17
        | 19
        | 21..=25
        | 35
        | 41..=49
        | 53
        | 54
        | 56..=58
        | 60..=62
        | 67
        | 73
        | 74
        | 80
        | 82
        | 84
        | 86..=89
        | 91
        | 95 => 255,
        _ => 0,
    }
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

    #[test]
    fn material_properties_match_every_native_registry_entry() {
        let fixture = include_str!("../../../tests/fixtures/light_material/native.tsv");
        let mut count = 0;
        for line in fixture.lines() {
            let row: Vec<u16> = line.split('\t').map(|word| word.parse().unwrap()).collect();
            assert_eq!(row.len(), 4);
            assert_eq!(usize::from(row[0]), count);
            let id = u8::try_from(row[0]).unwrap();
            assert_eq!(u16::from(opacity_level(id)), row[2], "opacity for {id}");
            assert_eq!(u16::from(emission_level(id)), row[3], "emission for {id}");
            count += 1;
        }
        assert_eq!(count, 256);
    }

    #[test]
    fn all_native_light_fixture_rows_match_raw_values() {
        let inputs = include_str!("../../../tests/fixtures/light/input.tsv");
        let expected = include_str!("../../../tests/fixtures/light/reference.tsv");
        let input_rows = inputs.lines().count();
        assert!(
            input_rows >= 50_000,
            "native fixture unexpectedly short: {input_rows}"
        );
        assert_eq!(input_rows, expected.lines().count());

        for (input, expected) in inputs.lines().zip(expected.lines()) {
            let input: Vec<_> = input.split('\t').collect();
            let expected: Vec<_> = expected.split('\t').collect();
            assert_eq!(input[0], expected[0]);
            assert_eq!(input[1], expected[1], "case {}", input[0]);
            match input[1] {
                "sample" => {
                    let dimension = match input[2] {
                        "0" => LightDimension::Overworld,
                        "-1" => LightDimension::Nether,
                        other => panic!("unknown fixture dimension {other}"),
                    };
                    let block = input[3].parse().unwrap();
                    let sky = input[4].parse().unwrap();
                    let subtraction = input[5].parse().unwrap();
                    let minimum = input[6].parse().unwrap();
                    let level = combined_light(block, sky, subtraction).unwrap();
                    let value =
                        sample_brightness(block, sky, subtraction, minimum, dimension).unwrap();
                    assert_eq!(level.to_string(), expected[2], "level {}", input[0]);
                    assert_eq!(
                        value.to_bits(),
                        u32::from_str_radix(expected[3], 16).unwrap(),
                        "brightness {}",
                        input[0]
                    );
                }
                "time" => {
                    let dimension = match input[2] {
                        "0" => LightDimension::Overworld,
                        "-1" => LightDimension::Nether,
                        other => panic!("unknown fixture dimension {other}"),
                    };
                    let time = input[3].parse().unwrap();
                    let partial = f32::from_bits(u32::from_str_radix(input[4], 16).unwrap());
                    let angle = celestial_angle(time, partial, dimension).unwrap();
                    let subtraction = sky_subtraction(time, partial, dimension).unwrap();
                    assert_eq!(
                        angle.to_bits(),
                        u32::from_str_radix(expected[2], 16).unwrap(),
                        "celestial angle {}",
                        input[0]
                    );
                    assert_eq!(
                        subtraction.to_string(),
                        expected[3],
                        "sky subtraction {}",
                        input[0]
                    );
                }
                "emission" => {
                    let block_id = input[2].parse().unwrap();
                    assert_eq!(
                        emission_level(block_id).to_string(),
                        expected[2],
                        "emission {}",
                        input[0]
                    );
                }
                other => panic!("unknown fixture operation {other}"),
            }
        }
    }
}
