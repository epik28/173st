//! Held-item wear on native block destruction. Does not decide whether the
//! player can harvest a block, or whether an attempted break succeeded.

use crate::items::{ItemKind, ItemStack};

/// The native `ItemStack.onBlockDestroyed` wear for registered item IDs.
/// `block_id` must identify the block that was actually destroyed.
pub fn wear_for_block(item_id: u16, block_id: u16) -> Option<u16> {
    if !(256..=359).contains(&item_id) || !(1..=255).contains(&block_id) {
        return None;
    }
    Some(match item_id {
        267 | 268 | 272 | 276 | 283 => 2,
        256..=258 | 269..=271 | 273..=275 | 277..=279 | 284..=286 => 1,
        359 if block_id == 18 || block_id == 30 => 1,
        _ => 0,
    })
}

/// Apply wear to a single supported stack after a successful block break.
/// Outer `None` means unsupported item/block; inner `None` means the tool broke.
pub fn after_block_break(stack: ItemStack, block_id: u16) -> Option<Option<ItemStack>> {
    let wear = wear_for_block(stack.id(), block_id)?;
    if wear == 0 {
        return Some(Some(stack));
    }
    let max = ItemKind::from_native_id(stack.id()).ok()?.max_damage();
    let damage = u32::from(stack.damage()) + u32::from(wear);
    if damage > u32::from(max) {
        return Some(None);
    }
    Some(ItemStack::new(stack.id(), stack.count(), damage as u16).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_wear_and_break_boundary_match_native() {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/block_wear/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<u16> = line.split('\t').map(|x| x.parse().unwrap()).collect();
            for (before, expected_damage, expected_count) in [(0, f[3], f[4]), (f[2], f[5], f[6])] {
                let stack = ItemStack::new(f[0], 1, before).unwrap();
                let result = after_block_break(stack, f[1]).unwrap();
                assert_eq!(
                    result.map_or(0, |s| u16::from(s.count())),
                    expected_count,
                    "{line}"
                );
                assert_eq!(result.map_or(0, |s| s.damage()), expected_damage, "{line}");
            }
            count += 1;
        }
        assert_eq!(count, 9984);
        assert!(wear_for_block(1, 1).is_none());
        assert!(wear_for_block(267, 0).is_none());
    }
}
