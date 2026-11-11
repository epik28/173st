//! Read-only extraction of the supported main inventory from Beta player NBT.

use beta_core::{
    items::{Inventory, ItemStack, MAIN_SLOT_COUNT},
    nbt::Tag,
};

/// Decode the native player Pos into the motion controller's bottom-center point.
///
/// Native `sn.d(nu)` writes Y as entity position plus ySize. The player's
/// bounding box bottom is that value minus its 1.62f yOffset. Require every
/// chunk touched by the player box to be loaded; otherwise callers use their
/// existing safe spawn. Every intersected cell must be air; motion is not restored.
pub fn position_from_player(
    player: Option<&Tag>,
    mut chunk_loaded: impl FnMut(i32, i32) -> bool,
    mut block_is_air: impl FnMut(i32, i32, i32) -> bool,
) -> Option<[f64; 3]> {
    let Tag::Compound(fields) = player? else {
        return None;
    };
    let Tag::List {
        element_type: beta_core::nbt::TagType::Double,
        values,
    } = field(fields, "Pos")?
    else {
        return None;
    };
    let [Tag::Double(x), Tag::Double(y), Tag::Double(z)] = values.as_slice() else {
        return None;
    };
    let position = [*x, *y - f64::from(1.62_f32), *z];
    if !position.iter().all(|v| v.is_finite())
        || position[1] < 0.0
        || position[1] + f64::from(1.8_f32) > 128.0
    {
        return None;
    }
    let radius = f64::from(0.6_f32 / 2.0_f32);
    let chunk = |v: f64| -> Option<i32> {
        let value = (v / 16.0).floor();
        if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            None
        } else {
            Some(value as i32)
        }
    };
    let (min_x, max_x) = (chunk(position[0] - radius)?, chunk(position[0] + radius)?);
    let (min_z, max_z) = (chunk(position[2] - radius)?, chunk(position[2] + radius)?);
    for x in min_x..=max_x {
        for z in min_z..=max_z {
            if !chunk_loaded(x, z) {
                return None;
            }
        }
    }
    // Conservatively require air in every cell touched by the box. This also
    // rejects water and non-full block shapes, whose collision is not audited here.
    let block = |v: f64| -> Option<i32> {
        let value = v.floor();
        if value < f64::from(i32::MIN) || value > f64::from(i32::MAX) {
            None
        } else {
            Some(value as i32)
        }
    };
    for x in block(position[0] - radius)?..=block(position[0] + radius)? {
        for y in block(position[1])?..=block(position[1] + f64::from(1.8_f32))? {
            for z in block(position[2] - radius)?..=block(position[2] + radius)? {
                if !block_is_air(x, y, z) {
                    return None;
                }
            }
        }
    }
    Some(position)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SavedInventory {
    pub inventory: Inventory,
    pub unsupported_stacks: usize,
}

/// Return the saved Beta health in half-heart units. Invalid or missing values
/// use the native viewer fallback of twenty, while valid values are bounded to
/// the ten-heart display.
pub fn health_from_player(player: Option<&Tag>) -> i16 {
    let Some(Tag::Compound(fields)) = player else {
        return 20;
    };
    match field(fields, "Health") {
        Some(Tag::Short(health)) => (*health).clamp(0, 20),
        _ => 20,
    }
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

    fn player_pos(values: Vec<Tag>) -> Tag {
        Tag::Compound(vec![(
            "Pos".into(),
            Tag::List {
                element_type: beta_core::nbt::TagType::Double,
                values,
            },
        )])
    }

    #[test]
    fn saved_position_uses_native_eye_offset_and_checks_all_touched_chunks() {
        let player = player_pos(vec![
            Tag::Double(-0.1),
            Tag::Double(65.0),
            Tag::Double(16.1),
        ]);
        assert_eq!(
            position_from_player(
                Some(&player),
                |x, z| (-1..=0).contains(&x) && (0..=1).contains(&z),
                |_, _, _| true
            ),
            Some([-0.1, 65.0 - f64::from(1.62_f32), 16.1])
        );
        assert_eq!(
            position_from_player(Some(&player), |x, z| x == -1 && z == 1, |_, _, _| true),
            None
        );
    }

    #[test]
    fn saved_position_matches_native_player_nbt_roundtrip() {
        let row = include_str!("../../../tests/fixtures/saved_position/native.tsv")
            .lines()
            .nth(1)
            .unwrap();
        let words: Vec<f64> = row
            .split('\t')
            .map(|s| f64::from_bits(u64::from_str_radix(s, 16).unwrap()))
            .collect();
        let player = player_pos(words[..3].iter().copied().map(Tag::Double).collect());
        let position = position_from_player(Some(&player), |_, _| true, |_, _, _| true).unwrap();
        assert_eq!(position[1].to_bits(), words[3].to_bits());
        assert_eq!(
            position_from_player(
                Some(&player),
                |_, _| true,
                |x, y, z| (x, y, z) != (-1, 64, 16)
            ),
            None
        );
    }

    #[test]
    fn saved_position_rejects_malformed_nonfinite_and_out_of_world_values() {
        assert_eq!(
            position_from_player(None, |_, _| true, |_, _, _| true),
            None
        );
        for values in [
            vec![Tag::Double(0.0)],
            vec![Tag::Float(0.0), Tag::Double(64.0), Tag::Double(0.0)],
            vec![Tag::Double(f64::NAN), Tag::Double(64.0), Tag::Double(0.0)],
            vec![Tag::Double(f64::MAX), Tag::Double(64.0), Tag::Double(0.0)],
            vec![Tag::Double(0.0), Tag::Double(0.0), Tag::Double(0.0)],
            vec![Tag::Double(0.0), Tag::Double(128.0), Tag::Double(0.0)],
        ] {
            assert_eq!(
                position_from_player(Some(&player_pos(values)), |_, _| true, |_, _, _| true),
                None
            );
        }
    }

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

    #[test]
    fn health_defaults_for_missing_or_malformed_player_data() {
        assert_eq!(health_from_player(None), 20);
        assert_eq!(health_from_player(Some(&Tag::String("bad".into()))), 20);
        assert_eq!(health_from_player(Some(&Tag::Compound(vec![]))), 20);
        assert_eq!(
            health_from_player(Some(&Tag::Compound(vec![("Health".into(), Tag::Byte(3))]))),
            20
        );
    }

    #[test]
    fn health_clamps_signed_short_to_display_range() {
        for (raw, expected) in [
            (-1, 0),
            (0, 0),
            (1, 1),
            (2, 2),
            (3, 3),
            (19, 19),
            (20, 20),
            (21, 20),
        ] {
            assert_eq!(
                health_from_player(Some(&Tag::Compound(vec![(
                    "Health".into(),
                    Tag::Short(raw)
                )]))),
                expected
            );
        }
    }
}
