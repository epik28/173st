//! Damage requested by the native living-entity landing handler.
//!
//! This only computes the requested damage; armor, invulnerability, health,
//! landing detection, sound, and water handling are separate behaviors.

/// Return the damage requested for a finite, nonnegative fall distance, or
/// `None` for invalid input. Arithmetic follows native `ls.b(float)`: subtract
/// three as `f32`, then take the ceiling and clamp nonpositive damage to zero.
pub fn landing_damage(distance: f32) -> Option<i32> {
    if !distance.is_finite() || distance < 0.0 {
        return None;
    }
    Some(((distance - 3.0).ceil() as i32).max(0))
}

#[cfg(test)]
mod tests {
    use super::landing_damage;

    #[test]
    fn native_living_entity_damage_calls_match_fixture() {
        let fixture = include_str!("../../../tests/fixtures/fall_damage/native.tsv");
        let mut cases = 0;
        for row in fixture.lines().skip(1) {
            let (bits, damage) = row.split_once('\t').unwrap();
            let distance = f32::from_bits(u32::from_str_radix(bits, 16).unwrap());
            assert_eq!(
                landing_damage(distance),
                Some(damage.parse().unwrap()),
                "{bits}"
            );
            cases += 1;
        }
        assert_eq!(cases, 11);
        assert_eq!(landing_damage(f32::NAN), None);
        assert_eq!(landing_damage(f32::INFINITY), None);
        assert_eq!(landing_damage(-1.0), None);
    }
}
