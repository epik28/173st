//! MathHelper-compatible functions used by the Beta 1.7.3 game.

const SIN_TABLE_SIZE: usize = 65_536;
const SIN_SCALE: f32 = 10_430.378_f32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MathError {
    DivisionByZero,
}

impl std::fmt::Display for MathError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::DivisionByZero => formatter.write_str("division by zero"),
        }
    }
}

impl std::error::Error for MathError {}

fn sin_table() -> &'static [f32; SIN_TABLE_SIZE] {
    static TABLE: std::sync::OnceLock<[f32; SIN_TABLE_SIZE]> = std::sync::OnceLock::new();
    TABLE.get_or_init(|| {
        let mut table = [0.0; SIN_TABLE_SIZE];
        for (index, value) in table.iter_mut().enumerate() {
            let angle = index as f64 * std::f64::consts::PI * 2.0 / SIN_TABLE_SIZE as f64;
            *value = angle.sin() as f32;
        }
        table
    })
}

#[inline]
fn lut_index(radians: f32) -> usize {
    ((radians * SIN_SCALE) as i32 as u32 as usize) & (SIN_TABLE_SIZE - 1)
}

/// Returns the original game's table-lookup sine approximation.
pub fn sin(radians: f32) -> f32 {
    sin_table()[lut_index(radians)]
}

/// Returns the original game's table-lookup cosine approximation.
pub fn cos(radians: f32) -> f32 {
    let index =
        ((radians * SIN_SCALE + 16_384.0_f32) as i32 as u32 as usize) & (SIN_TABLE_SIZE - 1);
    sin_table()[index]
}

/// Square root using the `f32` MathHelper overload.
pub fn sqrt_f32(value: f32) -> f32 {
    value.sqrt()
}

/// Square root using the `f64` overload, whose result is narrowed to `f32`.
pub fn sqrt_f64_to_f32(value: f64) -> f32 {
    value.sqrt() as f32
}

/// Java-cast-compatible floor for `f32`, including NaN and infinities.
pub fn floor_f32(value: f32) -> i32 {
    let truncated = value as i32;
    if value < truncated as f32 {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// Java-cast-compatible floor for `f64`, including NaN and infinities.
pub fn floor_f64(value: f64) -> i32 {
    let truncated = value as i32;
    if value < truncated as f64 {
        truncated.wrapping_sub(1)
    } else {
        truncated
    }
}

/// Absolute value matching the original comparison-and-negation method.
pub fn abs_f32(value: f32) -> f32 {
    if value >= 0.0 { value } else { -value }
}

/// Returns the larger absolute value using the original comparison behavior.
pub fn abs_max_f64(first: f64, second: f64) -> f64 {
    let first = if first < 0.0 { -first } else { first };
    let second = if second < 0.0 { -second } else { second };
    if first > second { first } else { second }
}

/// Computes the original MathHelper signed division variant without panicking.
pub fn floor_div_i32(dividend: i32, divisor: i32) -> Result<i32, MathError> {
    if divisor == 0 {
        return Err(MathError::DivisionByZero);
    }
    Ok(if dividend < 0 {
        let adjusted = dividend.wrapping_neg().wrapping_sub(1);
        adjusted
            .wrapping_div(divisor)
            .wrapping_neg()
            .wrapping_sub(1)
    } else {
        dividend / divisor
    })
}

/// Returns whether an optional string is absent or empty.
pub fn is_null_or_empty(value: Option<&str>) -> bool {
    match value {
        None => true,
        Some(text) => text.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_corpus_matches_all_public_operations_by_raw_bits() {
        for (input, expected) in include_str!("../../../tests/fixtures/math/input.tsv")
            .lines()
            .zip(include_str!("../../../tests/fixtures/math/reference.tsv").lines())
        {
            let fields: Vec<_> = input.split('\t').collect();
            let result = match fields[1] {
                "sin" => format!(
                    "{:08x}",
                    sin(f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap())).to_bits()
                ),
                "cos" => format!(
                    "{:08x}",
                    cos(f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap())).to_bits()
                ),
                "sqrt_f32" => format!(
                    "{:08x}",
                    sqrt_f32(f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap())).to_bits()
                ),
                "sqrt_f64_to_f32" => format!(
                    "{:08x}",
                    sqrt_f64_to_f32(f64::from_bits(u64::from_str_radix(fields[2], 16).unwrap()))
                        .to_bits()
                ),
                "floor_f32" => {
                    floor_f32(f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap()))
                        .to_string()
                }
                "floor_f64" => {
                    floor_f64(f64::from_bits(u64::from_str_radix(fields[2], 16).unwrap()))
                        .to_string()
                }
                "abs_f32" => format!(
                    "{:08x}",
                    abs_f32(f32::from_bits(u32::from_str_radix(fields[2], 16).unwrap())).to_bits()
                ),
                "abs_max_f64" => format!(
                    "{:016x}",
                    abs_max_f64(
                        f64::from_bits(u64::from_str_radix(fields[2], 16).unwrap()),
                        f64::from_bits(u64::from_str_radix(fields[3], 16).unwrap()),
                    )
                    .to_bits()
                ),
                "floor_div_i32" => {
                    let a = fields[2].parse().unwrap();
                    let b = fields[3].parse().unwrap();
                    match floor_div_i32(a, b) {
                        Ok(value) => value.to_string(),
                        Err(MathError::DivisionByZero) => "error_arithmetic".to_owned(),
                    }
                }
                "null_or_empty" => {
                    let empty = if fields[2] == "null" {
                        is_null_or_empty(None)
                    } else {
                        let bytes = decode_hex(fields[2]);
                        is_null_or_empty(Some(std::str::from_utf8(&bytes).unwrap()))
                    };
                    if empty { "1" } else { "0" }.to_owned()
                }
                operation => panic!("unexpected corpus operation {operation}"),
            };
            assert_eq!(
                format!("{}\t{}", fields[0], result),
                expected,
                "case {}",
                fields[0]
            );
        }
    }

    #[test]
    fn floor_division_matches_native_signed_behavior_and_zero_error() {
        assert_eq!(floor_div_i32(1, 0), Err(MathError::DivisionByZero));
        assert_eq!(floor_div_i32(i32::MIN, -1), Ok(2_147_483_646));
        for (a, b, expected) in [(-7, 3, -3), (7, -3, -2), (-7, -3, 1), (7, 3, 2)] {
            assert_eq!(floor_div_i32(a, b), Ok(expected));
        }
    }

    #[test]
    fn floor_handles_cast_boundaries_and_negative_fraction() {
        assert_eq!(floor_f32(-0.25), -1);
        assert_eq!(floor_f64(-0.25), -1);
        assert_eq!(floor_f32(f32::NAN), 0);
        assert_eq!(floor_f64(f64::INFINITY), i32::MAX);
        assert_eq!(floor_f64(f64::NEG_INFINITY), i32::MAX);
    }

    #[test]
    fn null_empty_and_nonempty_strings() {
        assert!(is_null_or_empty(None));
        assert!(is_null_or_empty(Some("")));
        assert!(!is_null_or_empty(Some(" ")));
        assert!(!is_null_or_empty(Some("雪")));
    }

    fn decode_hex(text: &str) -> Vec<u8> {
        text.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    b'A'..=b'F' => byte - b'A' + 10,
                    _ => panic!("invalid hex in committed corpus"),
                };
                digit(pair[0]) * 16 + digit(pair[1])
            })
            .collect()
    }
}
