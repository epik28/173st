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

/// Apply one native vertical displacement to the fall accumulator. The
/// returned distance is the pre-landing value passed to the damage handler.
/// Invalid or non-finite input is rejected without modifying the accumulator.
pub fn accumulate_fall(distance: &mut f32, delta_y: f64, on_ground: bool) -> Option<f32> {
    if !distance.is_finite() || *distance < 0.0 || !delta_y.is_finite() {
        return None;
    }
    if on_ground {
        let landed = (*distance > 0.0).then_some(*distance);
        *distance = 0.0;
        landed
    } else {
        if delta_y < 0.0 {
            *distance = (f64::from(*distance) - delta_y) as f32;
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::{accumulate_fall, landing_damage};

    #[test]
    fn native_fall_accumulation_and_landings_match_fixture() {
        let fixture = include_str!("../../../tests/fixtures/fall_damage/native_landing.tsv");
        let mut distance = 0.0;
        let mut calls = 0;
        let mut landed = 0.0_f32;
        for row in fixture.lines().skip(1) {
            let f: Vec<_> = row.split('\t').collect();
            let delta = f64::from_bits(u64::from_str_radix(f[1], 16).unwrap());
            if let Some(value) = accumulate_fall(&mut distance, delta, f[2] == "1") {
                calls += 1;
                landed = value;
            }
            assert_eq!(distance.to_bits(), u32::from_str_radix(f[3], 16).unwrap());
            assert_eq!(calls, f[4].parse::<usize>().unwrap());
            assert_eq!(landed.to_bits(), u32::from_str_radix(f[5], 16).unwrap());
        }
    }

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
