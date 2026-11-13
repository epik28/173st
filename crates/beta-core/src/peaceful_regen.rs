//! Native peaceful-difficulty player regeneration, isolated from runtime ticks.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RegenResult {
    pub health: i32,
    pub hurt_timer: i32,
    pub healed: bool,
}

/// Apply the native `gs.o()` peaceful-heal prefix to valid player state.
/// `entity_ticks` is the native entity tick counter at the moment of the call.
/// This does not advance the counter or implement other living tick behavior.
pub fn peaceful_tick(
    difficulty: u8,
    entity_ticks: i32,
    health: i32,
    hurt_timer: i32,
    max_hurt_timer: i32,
) -> Option<RegenResult> {
    if difficulty > 3 || !(0..=20).contains(&health) || hurt_timer < 0 || max_hurt_timer < 0 {
        return None;
    }
    let healed = difficulty == 0 && entity_ticks % 20 == 0 && (1..20).contains(&health);
    Some(RegenResult {
        health: health + i32::from(healed),
        hurt_timer: if healed {
            max_hurt_timer / 2
        } else {
            hurt_timer
        },
        healed,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_living_tick_decrements_timer_before_peaceful_heal() {
        for row in include_str!("../../../tests/fixtures/peaceful_regen/native_phase.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<i32> = row.split('\t').map(|v| v.parse().unwrap()).collect();
            let timer = (f[4] - 1).max(0);
            let state = peaceful_tick(0, f[0].wrapping_add(1), f[2], timer, 20).unwrap();
            assert_eq!(
                (f[0].wrapping_add(1), state.health, state.hurt_timer),
                (f[1], f[3], f[5])
            );
        }
    }

    #[test]
    fn native_player_peaceful_regeneration_and_timer() {
        let mut count = 0;
        for row in include_str!("../../../tests/fixtures/peaceful_regen/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<i32> = row.split('\t').map(|v| v.parse().unwrap()).collect();
            let result = peaceful_tick(f[0] as u8, f[1], f[2], f[3], 20).unwrap();
            assert_eq!((result.health, result.hurt_timer), (f[4], f[5]), "{row}");
            count += 1;
        }
        assert_eq!(count, 96);
        assert!(peaceful_tick(4, 0, 20, 0, 20).is_none());
        assert!(peaceful_tick(0, 0, -1, 0, 20).is_none());
    }
}
