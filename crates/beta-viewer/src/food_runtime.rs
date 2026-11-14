//! Selected-slot native food use in the disposable viewer inventory.
use beta_core::{
    fall_damage::UnarmoredHealth,
    food::use_food,
    items::{Inventory, ItemStack},
};

/// Targets whose native block interaction was observed to decline activation
/// before the held food is used. Unknown block states remain unsupported.
pub fn passive_target(target: Option<(u8, u8)>) -> bool {
    match target {
        None => true,
        Some((id, metadata)) if metadata <= 15 => matches!(
            id,
            1 | 2
                | 3
                | 4
                | 5
                | 12
                | 13
                | 17
                | 18
                | 20
                | 24
                | 31
                | 35
                | 44
                | 49
                | 78
                | 81
                | 82
                | 86
                | 87
                | 88
                | 89
                | 91
        ),
        _ => false,
    }
}

/// Returns `None` for a non-food item or unsupported state. Updates are atomic.
pub fn use_selected_food(inventory: &mut Inventory, health: &mut UnarmoredHealth) -> Option<()> {
    let index = usize::from(inventory.selected_hotbar());
    let stack = inventory.stack_at(index).ok()??;
    let result = use_food(
        stack.id(),
        stack.count(),
        u8::try_from(health.health).ok()?,
        u16::try_from(health.hurt_timer).ok()?,
        u16::try_from(health.max_hurt_timer).ok()?,
    )?;
    let replacement = if result.returned_count == 0 {
        None
    } else {
        Some(ItemStack::new(result.returned_id, result.returned_count, 0).ok()?)
    };
    inventory.set_slot(index, replacement).ok()?;
    health.health = i32::from(result.health);
    health.hurt_timer = i32::from(result.hurt_timer);
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn passive_block_activation_before_food_matches_native() {
        let mut count = 0;
        for row in include_str!("../../../tests/fixtures/food_passive/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<u16> = row
                .split('\t')
                .map(|value| value.parse().unwrap())
                .collect();
            assert_eq!(
                passive_target(Some((f[0] as u8, f[1] as u8))),
                f[3] == 0,
                "{row}"
            );
            assert_eq!((f[4], f[5], f[6]), (1, 14, 1), "{row}");
            count += 1;
        }
        assert_eq!(count, 2208);
        assert!(passive_target(None));
        assert!(!passive_target(Some((54, 0))));
        assert!(passive_target(Some((17, 1))));
        assert!(!passive_target(Some((17, 16))));
    }
    #[test]
    fn selected_slot_use_matches_native_controller() {
        for line in include_str!("../../../tests/fixtures/food/native_route.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split('\t').collect();
            let mut inventory = Inventory::new();
            inventory.select_hotbar(4).unwrap();
            inventory
                .set_slot(
                    4,
                    Some(ItemStack::new(f[0].parse().unwrap(), f[1].parse().unwrap(), 0).unwrap()),
                )
                .unwrap();
            let mut health = UnarmoredHealth {
                health: f[2].parse().unwrap(),
                previous_health: 0,
                hurt_timer: f[3].parse().unwrap(),
                max_hurt_timer: 20,
                last_damage: 0,
                hurt_animation: 0,
            };
            assert_eq!(use_selected_food(&mut inventory, &mut health), Some(()));
            assert_eq!(health.health, f[5].parse().unwrap());
            assert_eq!(health.hurt_timer, f[6].parse().unwrap());
            let slot = inventory
                .stack_at(4)
                .unwrap()
                .map(|s| format!("{}:{}", s.id(), s.count()))
                .unwrap_or("-".into());
            assert_eq!(slot, f[7]);
        }
    }
}
