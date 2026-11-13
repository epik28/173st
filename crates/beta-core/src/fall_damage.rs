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

/// Captured portion of the unarmored player's damage state. Timer advancement,
/// armor, multiplayer, death handling, sound and statistics are outside this
/// model; callers supply the current timer before each damage event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnarmoredHealth {
    pub health: i32,
    pub previous_health: i32,
    pub hurt_timer: i32,
    pub max_hurt_timer: i32,
    pub last_damage: i32,
    pub hurt_animation: i32,
}

impl UnarmoredHealth {
    /// Apply a fall damage request in 0..=253 (distances through 256 blocks),
    /// returning health lost. Unsupported input is rejected atomically.
    pub fn apply_fall_damage(&mut self, damage: i32) -> Option<i32> {
        if !(0..=253).contains(&damage)
            || self.hurt_timer < 0
            || self.max_hurt_timer < 0
            || self.last_damage < 0
        {
            return None;
        }
        if damage == 0 || self.health <= 0 {
            return Some(0);
        }
        let during_cooldown = (self.hurt_timer as f32) > (self.max_hurt_timer as f32) / 2.0;
        let applied = if during_cooldown {
            if damage <= self.last_damage {
                return Some(0);
            }
            damage - self.last_damage
        } else {
            damage
        };
        let health = self.health.checked_sub(applied)?;
        if !during_cooldown {
            self.previous_health = self.health;
            self.hurt_timer = self.max_hurt_timer;
            self.hurt_animation = 10;
        }
        self.last_damage = damage;
        self.health = health;
        Some(applied)
    }
}

#[cfg(test)]
mod tests {
    use super::{UnarmoredHealth, accumulate_fall, landing_damage};

    #[test]
    fn unarmored_fall_health_matches_native_cooldown_scenarios() {
        let fixture = include_str!("../../../tests/fixtures/fall_damage/native_health.tsv");
        let mut state = UnarmoredHealth {
            health: 20,
            previous_health: 0,
            hurt_timer: 0,
            max_hurt_timer: 20,
            last_damage: 0,
            hurt_animation: 0,
        };
        for row in fixture.lines().skip(1) {
            let f: Vec<_> = row.split('\t').collect();
            let number = |i: usize| f[i].parse::<i32>().unwrap();
            state.hurt_timer = number(5);
            assert_eq!(
                (state.health, state.last_damage, state.previous_health),
                (number(2), number(8), number(10)),
                "{}",
                f[0]
            );
            let distance = f32::from_bits(u32::from_str_radix(f[1], 16).unwrap());
            let applied = state.apply_fall_damage(landing_damage(distance).unwrap());
            assert_eq!(applied, Some(number(4)), "{}", f[0]);
            assert_eq!(
                (
                    state.health,
                    state.hurt_timer,
                    state.max_hurt_timer,
                    state.last_damage,
                    state.previous_health,
                    state.hurt_animation
                ),
                (
                    number(3),
                    number(6),
                    number(7),
                    number(9),
                    number(11),
                    number(12)
                ),
                "{}",
                f[0]
            );
        }
        let before = state;
        assert_eq!(state.apply_fall_damage(0), Some(0));
        assert_eq!(state.apply_fall_damage(-1), None);
        assert_eq!(state, before);
        state.health = 0;
        assert_eq!(state.apply_fall_damage(4), Some(0));
    }

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
