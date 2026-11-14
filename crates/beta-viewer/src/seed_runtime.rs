//! Wheat seed placement on farmland in the disposable scene.
use beta_core::items::ItemStack;

/// Outer `None` is an unsupported target; inner `None` consumes the last seed.
pub fn planting_result(
    stack: ItemStack,
    target: u8,
    above: u8,
    side: u8,
) -> Option<Option<ItemStack>> {
    if stack.id() != 295 || target != 60 || above != 0 || side != 1 {
        return None;
    }
    if stack.count() == 1 {
        Some(None)
    } else {
        Some(Some(
            ItemStack::new(295, stack.count() - 1, stack.damage()).ok()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planting_and_consumption_match_native() {
        let mut count = 0;
        for row in include_str!("../../../tests/fixtures/seed_use/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = row.split('\t').collect();
            let numbers: Vec<u16> = f[..5].iter().map(|s| s.parse().unwrap()).collect();
            let stack = ItemStack::new(295, 4, 0).unwrap();
            let result =
                planting_result(stack, numbers[0] as u8, numbers[2] as u8, numbers[3] as u8);
            assert_eq!(result.is_some(), numbers[4] == 1, "{row}");
            if let Some(Some(item)) = result {
                assert_eq!(
                    (f[6], item.count()),
                    ("59:0", f[7].parse().unwrap()),
                    "{row}"
                );
            }
            count += 1;
        }
        assert_eq!(count, 600);
        assert_eq!(
            planting_result(ItemStack::new(295, 1, 0).unwrap(), 60, 0, 1),
            Some(None)
        );
    }
}
