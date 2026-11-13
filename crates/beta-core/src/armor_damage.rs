//! Isolated native player armor damage reduction and fractional remainder.
//! Armor scoring, item wear, breakage and event ordering are not modeled here.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorReduction {
    pub health_damage: u16,
    /// Damage value passed by the original player method to its wear handler.
    pub wear_request: u16,
    pub remainder: u8,
}

/// Apply the captured `gs.b(int)` arithmetic for normal armor scores and
/// bounded damage. `remainder` carries fractional armor reduction between hits.
pub fn reduce_damage(damage: u16, armor_score: u8, remainder: u8) -> Option<ArmorReduction> {
    if damage > 253 || armor_score > 20 || remainder >= 25 {
        return None;
    }
    let weighted = u32::from(damage) * u32::from(25 - armor_score) + u32::from(remainder);
    Some(ArmorReduction {
        health_damage: (weighted / 25) as u16,
        wear_request: damage,
        remainder: (weighted % 25) as u8,
    })
}

#[cfg(test)]
mod tests {
    use super::reduce_damage;

    #[test]
    fn native_player_armor_reduction_matches_sequential_hits() {
        let fixture = include_str!("../../../tests/fixtures/armor_damage/native.tsv");
        let mut count = 0;
        for row in fixture.lines().skip(1) {
            let f: Vec<_> = row.split('\t').collect();
            let score = f[1].parse().unwrap();
            let damage = f[2].parse().unwrap();
            let hp_before: u16 = f[3].parse().unwrap();
            let hp_after: u16 = f[4].parse().unwrap();
            let remainder = f[5].parse().unwrap();
            let result = reduce_damage(damage, score, remainder).unwrap();
            assert_eq!(result.health_damage, hp_before - hp_after, "{}", f[0]);
            assert_eq!(result.remainder, f[6].parse().unwrap());
            assert_eq!(result.wear_request, f[7].parse().unwrap());
            count += 1;
        }
        assert_eq!(count, 20);
        assert!(reduce_damage(254, 0, 0).is_none());
        assert!(reduce_damage(1, 21, 0).is_none());
        assert!(reduce_damage(1, 0, 25).is_none());
    }
}
