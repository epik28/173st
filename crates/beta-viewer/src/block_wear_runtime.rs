//! Apply item wear only after a block edit has succeeded.
use beta_core::{block_wear::after_block_break, items::Inventory};

pub fn wear_selected_tool(inventory: &mut Inventory, block_id: u16) {
    let Some(stack) = inventory.selected_stack() else {
        return;
    };
    let Some(replacement) = after_block_break(stack, block_id) else {
        return;
    };
    // Selection is always a valid hotbar index, hence also a valid main slot.
    inventory
        .set_slot(usize::from(inventory.selected_hotbar()), replacement)
        .expect("selected hotbar index is valid");
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_core::items::ItemStack;

    #[test]
    fn successful_break_wears_only_selected_slot_and_can_break_tool() {
        let mut inventory = Inventory::new();
        inventory
            .set_slot(0, Some(ItemStack::new(267, 1, 0).unwrap()))
            .unwrap();
        inventory
            .set_slot(1, Some(ItemStack::new(257, 1, 249).unwrap()))
            .unwrap();
        inventory.select_hotbar(1).unwrap();
        wear_selected_tool(&mut inventory, 1);
        assert_eq!(inventory.selected_stack().unwrap().damage(), 250);
        assert_eq!(inventory.stack_at(0).unwrap().unwrap().damage(), 0);
        wear_selected_tool(&mut inventory, 1);
        assert_eq!(inventory.selected_stack(), None);
    }

    #[test]
    fn shears_wear_on_leaves_but_not_stone() {
        let mut inventory = Inventory::new();
        inventory
            .set_slot(0, Some(ItemStack::new(359, 1, 0).unwrap()))
            .unwrap();
        wear_selected_tool(&mut inventory, 1);
        assert_eq!(inventory.selected_stack().unwrap().damage(), 0);
        wear_selected_tool(&mut inventory, 18);
        assert_eq!(inventory.selected_stack().unwrap().damage(), 1);
    }
}
