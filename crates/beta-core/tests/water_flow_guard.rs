use beta_core::water::{
    WaterBlock, WaterCell, WaterError, WaterTick, WaterTickInput, transition_water_ticks,
};
use std::collections::BTreeMap;

#[test]
fn obstructed_two_step_scene_is_rejected_atomically() {
    for obstacle in [(2, 64, 0), (1, 64, 1), (1, 64, 0)] {
        let mut cells = BTreeMap::new();
        for y in 62..=65 {
            for x in -2..=2 {
                for z in -2..=2 {
                    let id = u8::from(y == 63 && (x, z) != (2, 0));
                    cells.insert((x, y, z), WaterCell::Loaded(WaterBlock { id, metadata: 0 }));
                }
            }
        }
        cells.insert(
            (0, 64, 0),
            WaterCell::Loaded(WaterBlock { id: 8, metadata: 0 }),
        );
        cells.insert(
            obstacle,
            WaterCell::Loaded(WaterBlock { id: 1, metadata: 0 }),
        );
        let input = WaterTickInput {
            world_time: 6,
            cells,
            queue: vec![WaterTick {
                position: (0, 64, 0),
                id: 8,
                due_time: 5,
            }],
            max_ticks: 64,
        };
        assert!(
            matches!(
                transition_water_ticks(input),
                Err(WaterError::UnsupportedTransition { .. })
            ),
            "obstacle {obstacle:?}"
        );
    }
}
