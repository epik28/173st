//! Native held-item melee damage and item wear, independent of entities.
//! Target damage, knockback, durability break handling and mob AI are separate.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AttackProfile {
    pub damage: u8,
    pub wear: u8,
}

/// Native unarmed, or item IDs 256 through 359 from the pinned item registry.
pub fn attack_profile(id: Option<u16>) -> Option<AttackProfile> {
    let Some(id) = id else {
        return Some(AttackProfile { damage: 1, wear: 0 });
    };
    if !(256..=359).contains(&id) {
        return None;
    }
    let (damage, wear) = match id {
        267 => (8, 1),
        268 | 283 => (4, 1),
        272 => (6, 1),
        276 => (10, 1),
        256 => (3, 2),
        257 => (4, 2),
        258 => (5, 2),
        269 | 284 => (1, 2),
        270 | 273 | 285 => (2, 2),
        271 | 274 | 286 => (3, 2),
        275 | 277 => (4, 2),
        278 => (5, 2),
        279 => (6, 2),
        _ => (1, 0),
    };
    Some(AttackProfile { damage, wear })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn item_attack_and_wear_match_native_inventory_dispatch() {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/attack_item/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<u16> = line.split('\t').map(|x| x.parse().unwrap()).collect();
            let profile = attack_profile(Some(f[0])).unwrap();
            assert_eq!(
                (u16::from(profile.damage), u16::from(profile.wear)),
                (f[1], f[3]),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 104);
        assert_eq!(attack_profile(None).unwrap().damage, 1);
        assert!(attack_profile(Some(360)).is_none());
    }
}
