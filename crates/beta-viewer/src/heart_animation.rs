//! Native Beta 1.7.3 HUD heart flash and low-health jitter state.
//! This computes sprite choices only; the renderer owns placement and timing.

use beta_core::java_random::JavaRandom;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HeartSprite {
    /// Extra logical pixels down, independently selected per heart at low health.
    pub y_offset: u8,
    /// Horizontal atlas coordinate of the heart background (y = 0).
    pub background_u: u8,
    /// Optional damage-flash overlay drawn between background and current health.
    pub previous_u: Option<u8>,
    /// Optional current-health foreground atlas coordinate.
    pub current_u: Option<u8>,
}

/// Evaluate one frame of the HUD. `gui_tick` is the original HUD update count,
/// not a render-frame count. `hurt_timer` is the player's native hurt-resistance
/// counter. Both integer multiplication and RNG seeding match Java overflow.
#[must_use]
pub fn heart_sprites(
    gui_tick: i32,
    health: i32,
    previous_health: i32,
    hurt_timer: i32,
) -> [HeartSprite; 10] {
    let flash = hurt_timer >= 10 && hurt_timer / 3 % 2 == 1;
    let mut rng = JavaRandom::new(i64::from(gui_tick.wrapping_mul(312_871)));
    std::array::from_fn(|i| {
        let threshold = i as i32 * 2 + 1;
        let sprite = |value, full, half| {
            if threshold < value {
                Some(full)
            } else if threshold == value {
                Some(half)
            } else {
                None
            }
        };
        HeartSprite {
            y_offset: if health <= 4 {
                rng.next_int_bound(2).expect("positive bound") as u8
            } else {
                0
            },
            background_u: if flash { 25 } else { 16 },
            previous_u: if flash {
                sprite(previous_health, 70, 79)
            } else {
                None
            },
            current_u: sprite(health, 52, 61),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_bytecode_derived_java_matrix() {
        for row in include_str!("../../../tests/fixtures/heart_hud/math.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<i32> = row.split('\t').map(|v| v.parse().unwrap()).collect();
            let sprite =
                heart_sprites(fields[0], fields[1], fields[2], fields[3])[fields[4] as usize];
            let optional = |v: Option<u8>| v.map(i32::from).unwrap_or(-1);
            assert_eq!(
                (
                    i32::from(sprite.y_offset),
                    i32::from(sprite.background_u),
                    optional(sprite.previous_u),
                    optional(sprite.current_u)
                ),
                (fields[5], fields[6], fields[7], fields[8]),
                "{row}"
            );
        }
    }
}
