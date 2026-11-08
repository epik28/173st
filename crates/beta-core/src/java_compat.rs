//! Primitive conversions and string hashing with Java semantics.

/// Converts an `f32` to `i32` using Java's saturating floating-point cast rules.
#[inline]
pub fn f32_to_i32(value: f32) -> i32 {
    value as i32
}

/// Converts an `f64` to `i32` using Java's saturating floating-point cast rules.
#[inline]
pub fn f64_to_i32(value: f64) -> i32 {
    value as i32
}

/// Converts an `f32` to `i64` using Java's saturating floating-point cast rules.
#[inline]
pub fn f32_to_i64(value: f32) -> i64 {
    value as i64
}

/// Converts an `f64` to `i64` using Java's saturating floating-point cast rules.
#[inline]
pub fn f64_to_i64(value: f64) -> i64 {
    value as i64
}

/// Computes `String.hashCode()` over UTF-16 code units with Java `i32` wrapping.
pub fn string_hash_code(value: &str) -> i32 {
    value.encode_utf16().fold(0_i32, |hash, unit| {
        hash.wrapping_mul(31).wrapping_add(i32::from(unit))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn decode_hex(value: &str) -> Vec<u8> {
        assert!(value.len().is_multiple_of(2), "fixture hex length");
        value
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    b'A'..=b'F' => byte - b'A' + 10,
                    _ => panic!("fixture contains non-hex string bytes"),
                };
                digit(pair[0]) * 16 + digit(pair[1])
            })
            .collect()
    }

    #[test]
    fn every_committed_java_fixture_row_matches() {
        let inputs = include_str!("../../../tests/fixtures/java/input.tsv");
        let reference = include_str!("../../../tests/fixtures/java/reference.tsv");
        assert_eq!(inputs.lines().count(), reference.lines().count());
        let mut count = 0;

        for (input, expected) in inputs.lines().zip(reference.lines()) {
            let fields: Vec<_> = input.split('\t').collect();
            assert_eq!(fields.len(), 3, "fixture input row: {input}");
            let result = match fields[1] {
                "f32_to_i32" => f32_to_i32(f32::from_bits(
                    u32::from_str_radix(fields[2], 16).expect("fixture f32 bits"),
                ))
                .to_string(),
                "f64_to_i32" => f64_to_i32(f64::from_bits(
                    u64::from_str_radix(fields[2], 16).expect("fixture f64 bits"),
                ))
                .to_string(),
                "f32_to_i64" => f32_to_i64(f32::from_bits(
                    u32::from_str_radix(fields[2], 16).expect("fixture f32 bits"),
                ))
                .to_string(),
                "f64_to_i64" => f64_to_i64(f64::from_bits(
                    u64::from_str_radix(fields[2], 16).expect("fixture f64 bits"),
                ))
                .to_string(),
                "string_hash_code" => {
                    let bytes = decode_hex(fields[2]);
                    string_hash_code(std::str::from_utf8(&bytes).expect("fixture UTF-8"))
                        .to_string()
                }
                operation => panic!("unknown fixture operation: {operation}"),
            };
            assert_eq!(
                format!("{}\t{}", fields[0], result),
                expected,
                "case {}",
                fields[0]
            );
            count += 1;
        }

        assert_eq!(count, 18_493);
    }

    #[test]
    fn float_casts_saturate_nan_and_truncate_toward_zero() {
        assert_eq!(f32_to_i32(f32::NAN), 0);
        assert_eq!(f64_to_i32(f64::NAN), 0);
        assert_eq!(f32_to_i64(f32::INFINITY), i64::MAX);
        assert_eq!(f64_to_i64(f64::NEG_INFINITY), i64::MIN);
        assert_eq!(f32_to_i32(-1.9), -1);
        assert_eq!(f64_to_i64(-1.9), -1);
    }

    #[test]
    fn string_hash_uses_utf16_units_and_wrapping_i32() {
        assert_eq!(string_hash_code(""), 0);
        assert_eq!(string_hash_code("abc"), 96_354);
        assert_eq!(string_hash_code("雪"), 38_634);
        assert_eq!(string_hash_code("😀"), 1_772_899);
    }
}
