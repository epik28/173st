//! Native hoe transformation for the verified air-above case.
use beta_core::items::{ItemKind, ItemStack};

/// `None` means this selected stack/target is outside the implemented case.
/// Inner `None` means the hoe broke after the successful use.
pub fn hoe_result(stack: ItemStack, target: u8, above: u8, side: u8) -> Option<Option<ItemStack>> {
    if !(290..=294).contains(&stack.id()) || above != 0 || side > 5 {
        return None;
    }
    if target != 3 && !(target == 2 && side != 0) {
        return None;
    }
    let max = ItemKind::from_native_id(stack.id()).ok()?.max_damage();
    let damage = stack.damage() + 1;
    if damage > max {
        Some(None)
    } else {
        Some(Some(
            ItemStack::new(stack.id(), stack.count(), damage).ok()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hoe_air_above_matches_native_and_tool_can_break() {
        for row in include_str!("../../../tests/fixtures/hoe/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<u16> = row.split('\t').map(|s| s.parse().unwrap()).collect();
            if f[2] != 0 {
                continue;
            }
            let stack = ItemStack::new(f[0], 1, 0).unwrap();
            let result = hoe_result(stack, f[1] as u8, 0, f[3] as u8);
            assert_eq!(result.is_some(), f[4] != 0, "{row}");
            if let Some(Some(item)) = result {
                assert_eq!((f[5], f[6], f[7], item.damage()), (60, 0, 1, 1), "{row}");
            }
        }
        let almost_broken = ItemStack::new(290, 1, 59).unwrap();
        assert_eq!(hoe_result(almost_broken, 3, 0, 1), Some(None));
        assert_eq!(hoe_result(almost_broken, 2, 1, 1), None);
    }
}
