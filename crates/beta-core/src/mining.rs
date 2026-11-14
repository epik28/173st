//! Beta block hardness, held item speed and per-tick breaking progress.
//! The caller still manages targeting, input, cooldown and block changes.
#[path = "mining_table.rs"]
mod table;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MiningProperties {
    pub hardness: f32,
    pub speed: f32,
    pub can_harvest: bool,
}

/// Captured native block and item registries. `None` item means bare hand.
pub fn properties(block_id: u8, item_id: Option<u16>) -> Option<MiningProperties> {
    let profile = match item_id.unwrap_or(0) {
        0..=255 | 259..=266 | 280..=282 | 287..=358 => 0,
        256 => 1,
        257 => 2,
        258 => 3,
        267 | 268 | 272 | 276 | 283 => 4,
        269 => 5,
        270 => 6,
        271 => 7,
        273 => 8,
        274 => 9,
        275 => 10,
        277 => 11,
        278 => 12,
        279 => 13,
        284 => 14,
        285 => 15,
        286 => 16,
        359 => 17,
        _ => return None,
    };
    let (_, hardness, profiles) = table::BLOCKS.iter().find(|row| row.0 == block_id)?;
    let encoded = profiles[profile];
    Some(MiningProperties {
        hardness: f32::from_bits(*hardness),
        speed: [1.0, 1.5, 2.0, 4.0, 5.0, 6.0, 8.0, 12.0, 15.0][usize::from(encoded & 15)],
        can_harvest: encoded & 16 != 0,
    })
}

/// Native `Block.blockStrength(player)`, preserving f32 operation order.
pub fn progress_per_tick(props: MiningProperties, underwater: bool, on_ground: bool) -> f32 {
    if props.hardness < 0.0 {
        return 0.0;
    }
    if !props.can_harvest {
        return 1.0 / props.hardness / 100.0;
    }
    let mut speed = props.speed;
    if underwater {
        speed /= 5.0;
    }
    if !on_ground {
        speed /= 5.0;
    }
    speed / props.hardness / 30.0
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn registry_properties_match_native() {
        let mut count = 0;
        for row in include_str!("../../../tests/fixtures/mining/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = row.split('\t').collect();
            let id = f[1].parse::<u16>().unwrap();
            let p = properties(f[0].parse().unwrap(), (id != 0).then_some(id)).unwrap();
            assert_eq!(
                p.hardness.to_bits(),
                u32::from_str_radix(f[2], 16).unwrap(),
                "{row}"
            );
            assert_eq!(
                p.speed.to_bits(),
                u32::from_str_radix(f[3], 16).unwrap(),
                "{row}"
            );
            assert_eq!(p.can_harvest, f[4] == "1", "{row}");
            for (column, (underwater, on_ground)) in
                [(false, true), (false, false), (true, true), (true, false)]
                    .into_iter()
                    .enumerate()
            {
                assert_eq!(
                    progress_per_tick(p, underwater, on_ground).to_bits(),
                    u32::from_str_radix(f[5 + column], 16).unwrap(),
                    "{row}"
                );
            }
            count += 1;
        }
        assert_eq!(count, 10080);
    }
    #[test]
    fn speed_penalties_and_unbreakable_blocks() {
        let p = properties(1, Some(257)).unwrap();
        assert_eq!(
            progress_per_tick(p, true, false).to_bits(),
            (p.speed / 5.0 / 5.0 / p.hardness / 30.0).to_bits()
        );
        assert_eq!(
            progress_per_tick(properties(7, None).unwrap(), false, true),
            0.0
        );
        assert!(progress_per_tick(properties(6, None).unwrap(), false, true).is_infinite());
    }
}
