//! Native Beta armor durability, inventory score and wear for the 20 armor IDs.
//! This module does not model item events or writing equipment to a save.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArmorPiece {
    pub id: u16,
    pub damage: u16,
}

impl ArmorPiece {
    pub fn new(id: u16, damage: u16) -> Option<Self> {
        let (_, max) = properties(id)?;
        (damage <= max).then_some(Self { id, damage })
    }
}

/// Native armor material does not alter protection, only durability.
pub fn properties(id: u16) -> Option<(u16, u16)> {
    let index = id.checked_sub(298)?;
    if index >= 20 {
        return None;
    }
    let kind = usize::from(index % 4);
    let material = usize::from(index / 4);
    let protection = [3, 8, 6, 3][kind];
    let max = [11, 16, 15, 13][kind] * [3, 6, 12, 24, 6][material];
    Some((protection, max))
}

/// Calculate `ix.f()` across all occupied slots. Invalid items are rejected.
pub fn armor_score(slots: &[Option<ArmorPiece>; 4]) -> Option<u16> {
    let mut protection = 0_u32;
    let mut max_total = 0_u32;
    let mut remaining = 0_u32;
    for piece in slots.iter().flatten() {
        let (value, max) = properties(piece.id)?;
        if piece.damage > max {
            return None;
        }
        protection += u32::from(value);
        max_total += u32::from(max);
        remaining += u32::from(max - piece.damage);
    }
    if max_total == 0 {
        return Some(0);
    }
    Some((((protection - 1) * remaining) / max_total + 1) as u16)
}

/// Apply the same durability damage to every armor piece, removing a piece
/// only once its damage exceeds its maximum, as `ix.e(int)` does.
pub fn apply_wear(slots: &mut [Option<ArmorPiece>; 4], amount: u16) -> Option<()> {
    let mut next = *slots;
    for slot in &mut next {
        let Some(piece) = slot else { continue };
        let (_, max) = properties(piece.id)?;
        if piece.damage > max {
            return None;
        }
        let damaged = u32::from(piece.damage) + u32::from(amount);
        if damaged > u32::from(max) {
            *slot = None;
        } else {
            piece.damage = damaged as u16;
        }
    }
    *slots = next;
    Some(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_registry_individual_scores_and_break_boundary() {
        let fixture = include_str!("../../../tests/fixtures/armor_damage/equipment.tsv");
        let mut count = 0;
        for row in fixture.lines().skip(1) {
            let f: Vec<u16> = row.split('\t').map(|v| v.parse().unwrap()).collect();
            assert_eq!(properties(f[0]), Some((f[1], f[2])));
            let mut slots = [Some(ArmorPiece::new(f[0], 0).unwrap()), None, None, None];
            assert_eq!(armor_score(&slots), Some(f[3]));
            slots[0].as_mut().unwrap().damage = f[2] / 2;
            assert_eq!(armor_score(&slots), Some(f[4]));
            slots[0].as_mut().unwrap().damage = f[2] - 1;
            apply_wear(&mut slots, 1).unwrap();
            assert_eq!(armor_score(&slots), Some(f[5]));
            assert_eq!(slots[0].unwrap().damage, f[6]);
            apply_wear(&mut slots, 1).unwrap();
            assert_eq!(u16::from(slots[0].is_none()), f[7]);
            count += 1;
        }
        assert_eq!(count, 20);
        assert_eq!(armor_score(&[None; 4]), Some(0));
    }

    #[test]
    fn native_mixed_armor_score_and_wear() {
        let mut slots = [
            ArmorPiece::new(298, 10),
            ArmorPiece::new(307, 80),
            ArmorPiece::new(313, 200),
            None,
        ];
        for row in include_str!("../../../tests/fixtures/armor_damage/equipment_mix.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = row.split('\t').collect();
            apply_wear(&mut slots, f[0].parse().unwrap()).unwrap();
            assert_eq!(armor_score(&slots), Some(f[1].parse().unwrap()));
            for (slot, expected) in slots.iter().zip(&f[2..]) {
                assert_eq!(slot.map(|p| p.damage), expected.parse().ok());
            }
        }
    }
}
