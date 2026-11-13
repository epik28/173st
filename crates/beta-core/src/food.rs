//! Native Beta instant-use food effects, without input or inventory routing.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FoodResult {
    pub health: u8,
    pub hurt_timer: u16,
    /// The native returned stack ID; a zero count means its slot becomes empty.
    pub returned_id: u16,
    pub returned_count: u8,
}

/// Use one supported native food item. Beta consumes food at full health too;
/// mushroom stew returns a bowl. A dead player's health and timer stay put.
pub fn use_food(
    id: u16,
    count: u8,
    health: u8,
    hurt_timer: u16,
    max_hurt_timer: u16,
) -> Option<FoodResult> {
    let (heal, limit) = match id {
        260 => (4, 1),
        282 => (10, 1),
        297 => (5, 1),
        319 => (3, 1),
        320 => (8, 1),
        322 => (42, 1),
        349 => (2, 1),
        350 => (5, 1),
        357 => (1, 8),
        _ => return None,
    };
    if count == 0 || count > limit || health > 20 {
        return None;
    }
    let alive = health > 0;
    Some(FoodResult {
        health: if alive {
            health.saturating_add(heal).min(20)
        } else {
            health
        },
        hurt_timer: if alive {
            max_hurt_timer / 2
        } else {
            hurt_timer
        },
        returned_id: if id == 282 { 281 } else { id },
        returned_count: if id == 282 { 1 } else { count - 1 },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_food_registry_and_use_match() {
        let mut count = 0;
        for line in include_str!("../../../tests/fixtures/food/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<u16> = line.split('\t').map(|x| x.parse().unwrap()).collect();
            let result = use_food(f[0], f[4] as u8, f[3] as u8, f[5], 20).unwrap();
            assert_eq!(
                (
                    u16::from(result.health),
                    result.hurt_timer,
                    result.returned_id,
                    u16::from(result.returned_count)
                ),
                (f[6], f[8], f[9], f[10]),
                "{line}"
            );
            count += 1;
        }
        assert_eq!(count, 45);
        assert_eq!(use_food(357, 8, 10, 0, 20).unwrap().returned_count, 7);
        assert!(use_food(357, 9, 10, 0, 20).is_none());
        assert!(use_food(1, 1, 10, 0, 20).is_none());
    }
}
