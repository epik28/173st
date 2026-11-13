//! Selected-slot native food use in the disposable viewer inventory.
use beta_core::{
    fall_damage::UnarmoredHealth,
    food::use_food,
    items::{Inventory, ItemStack},
};

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
