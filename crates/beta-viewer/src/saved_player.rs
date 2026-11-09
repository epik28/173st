//! Read-only extraction of the supported main inventory from Beta player NBT.

use beta_core::{
    items::{Inventory, ItemStack, MAIN_SLOT_COUNT},
    nbt::Tag,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedInventory {
    pub inventory: Inventory,
    pub unsupported_stacks: usize,
}

pub fn inventory_from_player(player: Option<&Tag>) -> SavedInventory {
    let mut inventory = Inventory::new();
    let mut unsupported_stacks = 0;
    let Some(Tag::Compound(fields)) = player else {
        return SavedInventory {
            inventory,
            unsupported_stacks,
        };
    };
    let Some(Tag::List {
        element_type: beta_core::nbt::TagType::Compound,
        values,
    }) = fields
        .iter()
        .find(|(name, _)| name == "Inventory")
        .map(|(_, value)| value)
    else {
        return SavedInventory {
            inventory,
            unsupported_stacks,
        };
    };

    for value in values {
        let Tag::Compound(stack_fields) = value else {
            continue;
        };
        let slot = match field(stack_fields, "Slot") {
            Some(Tag::Byte(slot)) => usize::try_from(*slot).ok(),
            _ => None,
        };
        let Some(slot) = slot.filter(|slot| *slot < MAIN_SLOT_COUNT) else {
            continue;
        };
        let (Some(Tag::Short(id)), Some(Tag::Byte(count)), Some(Tag::Short(damage))) = (
            field(stack_fields, "id"),
            field(stack_fields, "Count"),
            field(stack_fields, "Damage"),
        ) else {
            unsupported_stacks += 1;
            continue;
        };
        let stack = (u16::try_from(*id).ok())
            .zip(u8::try_from(*count).ok())
            .zip(u16::try_from(*damage).ok())
            .and_then(|((id, count), damage)| ItemStack::new(id, count, damage).ok());
        match stack {
            Some(stack) => {
                let _ = inventory.set_slot(slot, Some(stack));
            }
            None => unsupported_stacks += 1,
        }
    }
    SavedInventory {
        inventory,
        unsupported_stacks,
    }
}

fn field<'a>(fields: &'a [(String, Tag)], name: &str) -> Option<&'a Tag> {
    fields
        .iter()
        .find(|(key, _)| key == name)
        .map(|(_, value)| value)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stack(slot: i8, id: i16, count: i8, damage: i16) -> Tag {
        Tag::Compound(vec![
            ("Slot".into(), Tag::Byte(slot)),
            ("id".into(), Tag::Short(id)),
            ("Count".into(), Tag::Byte(count)),
            ("Damage".into(), Tag::Short(damage)),
        ])
    }

    #[test]
    fn loads_only_supported_main_inventory_items_from_player_nbt() {
        let player = Tag::Compound(vec![(
            "Inventory".into(),
            Tag::List {
                element_type: beta_core::nbt::TagType::Compound,
                values: vec![
                    stack(0, 3, 12, 0),
                    stack(35, 267, 1, 0),
                    stack(2, 999, 1, 0),
                    stack(100, 1, 1, 0),
                ],
            },
        )]);
        let loaded = inventory_from_player(Some(&player));
        assert_eq!(loaded.inventory.stack_at(0).unwrap().unwrap().id(), 3);
        assert_eq!(loaded.inventory.stack_at(0).unwrap().unwrap().count(), 12);
        assert_eq!(loaded.inventory.stack_at(35).unwrap().unwrap().id(), 267);
        assert_eq!(loaded.unsupported_stacks, 1);
    }

    #[test]
    fn malformed_or_missing_player_inventory_stays_empty() {
        assert_eq!(inventory_from_player(None).inventory, Inventory::new());
        let player = Tag::Compound(vec![("Inventory".into(), Tag::String("bad".into()))]);
        assert_eq!(
            inventory_from_player(Some(&player)).inventory,
            Inventory::new()
        );
    }
}
