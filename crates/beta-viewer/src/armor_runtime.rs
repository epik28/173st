//! In-memory bridge from supported saved armor to captured player damage logic.
//! This does not persist items, update equipment UI, or model death and respawn.

use beta_core::{
    armor_damage::reduce_damage,
    armor_equipment::{ArmorPiece, apply_wear, armor_score},
    fall_damage::UnarmoredHealth,
    nbt::{Tag, TagType},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArmorRuntime {
    pub health: UnarmoredHealth,
    pub slots: [Option<ArmorPiece>; 4],
    pub remainder: u8,
}

/// Read native inventory armor slots 100..103. Reject ambiguous or unsupported
/// equipment so callers never silently treat it as unarmored.
pub fn read_armor(player: Option<&Tag>) -> Option<[Option<ArmorPiece>; 4]> {
    let mut slots = [None; 4];
    let Some(player) = player else {
        return Some(slots);
    };
    let Tag::Compound(fields) = player else {
        return None;
    };
    let field = |name: &str| {
        fields
            .iter()
            .find(|(key, _)| key == name)
            .map(|(_, tag)| tag)
    };
    let Some(inventory) = field("Inventory") else {
        return Some(slots);
    };
    let Tag::List {
        element_type,
        values,
    } = inventory
    else {
        return None;
    };
    if *element_type != TagType::Compound && !(*element_type == TagType::End && values.is_empty()) {
        return None;
    }
    for entry in values {
        let Tag::Compound(fields) = entry else {
            return None;
        };
        let get = |name: &str| {
            fields
                .iter()
                .find(|(key, _)| key == name)
                .map(|(_, tag)| tag)
        };
        let Some(Tag::Byte(slot)) = get("Slot") else {
            return None;
        };
        if !(100..=103).contains(slot) {
            continue;
        }
        let index = usize::try_from(*slot - 100).ok()?;
        if slots[index].is_some() {
            return None;
        }
        let (Some(Tag::Short(id)), Some(Tag::Short(damage)), Some(Tag::Byte(1))) =
            (get("id"), get("Damage"), get("Count"))
        else {
            return None;
        };
        slots[index] = Some(ArmorPiece::new(
            u16::try_from(*id).ok()?,
            u16::try_from(*damage).ok()?,
        )?);
    }
    Some(slots)
}

impl ArmorRuntime {
    /// Apply captured null-attacker damage with native cooldown, armor score,
    /// carry and wear. Failure leaves every field unchanged.
    pub fn apply_damage(&mut self, damage: i32) -> Option<i32> {
        let mut next = self.clone();
        let before = next.health.health;
        let unarmored_damage = next.health.apply_fall_damage(damage)?;
        if unarmored_damage == 0 {
            *self = next;
            return Some(0);
        }
        let score = u8::try_from(armor_score(&next.slots)?).ok()?;
        let reduced = reduce_damage(u16::try_from(unarmored_damage).ok()?, score, next.remainder)?;
        apply_wear(&mut next.slots, reduced.wear_request)?;
        next.health.health = before.checked_sub(i32::from(reduced.health_damage))?;
        next.remainder = reduced.remainder;
        *self = next;
        Some(i32::from(reduced.health_damage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use beta_core::fall_damage::landing_damage;

    #[test]
    fn combined_cooldown_armor_wear_matches_original_player() {
        let mut runtime = ArmorRuntime {
            health: UnarmoredHealth {
                health: 20,
                previous_health: 0,
                hurt_timer: 0,
                max_hurt_timer: 20,
                last_damage: 0,
                hurt_animation: 0,
            },
            slots: [
                ArmorPiece::new(298, 31),
                ArmorPiece::new(307, 80),
                None,
                None,
            ],
            remainder: 0,
        };
        for line in include_str!("../../../tests/fixtures/armor_damage/native_health.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split('\t').collect();
            runtime.health.hurt_timer = f[2].parse().unwrap();
            runtime
                .apply_damage(landing_damage(f[1].parse().unwrap()).unwrap())
                .unwrap();
            assert_eq!(runtime.health.health, f[3].parse().unwrap());
            assert_eq!(runtime.health.hurt_timer, f[4].parse().unwrap());
            assert_eq!(runtime.health.last_damage, f[5].parse().unwrap());
            assert_eq!(runtime.remainder, f[6].parse().unwrap());
            assert_eq!(armor_score(&runtime.slots), Some(f[7].parse().unwrap()));
            assert_eq!(runtime.slots[0].map(|p| p.damage), f[8].parse().ok());
            assert_eq!(runtime.slots[1].map(|p| p.damage), f[9].parse().ok());
        }
    }

    #[test]
    fn nbt_armor_requires_valid_unique_armor_stacks() {
        let entry = |slot, id, damage| {
            Tag::Compound(vec![
                ("Slot".into(), Tag::Byte(slot)),
                ("id".into(), Tag::Short(id)),
                ("Damage".into(), Tag::Short(damage)),
                ("Count".into(), Tag::Byte(1)),
            ])
        };
        let player = |values| {
            Tag::Compound(vec![(
                "Inventory".into(),
                Tag::List {
                    element_type: TagType::Compound,
                    values,
                },
            )])
        };
        let valid = player(vec![entry(100, 298, 1), entry(9, 1, 0)]);
        assert_eq!(
            read_armor(Some(&valid)).unwrap()[0],
            ArmorPiece::new(298, 1)
        );
        assert!(read_armor(Some(&player(vec![entry(100, 298, 0), entry(100, 298, 0)]))).is_none());
        assert!(read_armor(Some(&player(vec![entry(100, 1, 0)]))).is_none());
        assert!(read_armor(Some(&player(vec![entry(100, 298, 34)]))).is_none());
    }
}
