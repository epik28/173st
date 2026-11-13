//! Isolated Beta 1.7.3 living-entity air counter.
//!
//! This models the native tick's air and emitted drowning event only; contact
//! detection, damage cooldown, particles and player integration belong to the
//! caller.

/// Result of one living-entity air update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AirTick {
    pub air: i32,
    /// Requested damage, before the target's damage and armor rules.
    pub damage: u8,
    /// Number of bubble particle requests.
    pub bubbles: u8,
}

/// Advance the native air counter by one living-entity tick.
///
/// `submerged` means the eye is inside water, not merely that the body touches
/// water. `breathes_underwater` corresponds to the entity's native exemption.
#[must_use]
pub fn tick_air(
    air: i32,
    max_air: i32,
    alive: bool,
    submerged: bool,
    breathes_underwater: bool,
) -> AirTick {
    if alive && submerged && !breathes_underwater {
        let next = air.wrapping_sub(1);
        if next == -20 {
            AirTick {
                air: 0,
                damage: 2,
                bubbles: 8,
            }
        } else {
            AirTick {
                air: next,
                damage: 0,
                bubbles: 0,
            }
        }
    } else {
        AirTick {
            air: max_air,
            damage: 0,
            bubbles: 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::tick_air;

    #[test]
    fn matches_native_living_tick_matrix() {
        for row in include_str!("../../../tests/fixtures/drowning/native.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = row.split('\t').collect();
            let result = tick_air(
                fields[0].parse().unwrap(),
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
                fields[4].parse().unwrap(),
            );
            assert_eq!(result.air, fields[5].parse::<i32>().unwrap(), "{row}");
            assert_eq!(result.damage, fields[6].parse::<u8>().unwrap(), "{row}");
            assert_eq!(result.bubbles, fields[7].parse::<u8>().unwrap(), "{row}");
        }
    }

    #[test]
    fn drowning_repeats_every_twenty_ticks_and_surface_refills() {
        let mut air = 300;
        let mut damage_ticks = Vec::new();
        for tick in 1..=360 {
            let result = tick_air(air, 300, true, true, false);
            air = result.air;
            if result.damage != 0 {
                damage_ticks.push(tick);
            }
        }
        assert_eq!(damage_ticks, [320, 340, 360]);
        assert_eq!(tick_air(air, 300, true, false, false).air, 300);
    }
}
