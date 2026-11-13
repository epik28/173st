//! Viewer adapter for the native player's water-at-eye query.

use beta_core::{chunk::BlockData, motion::MotionState};

/// `None` means that the eye cell is unavailable, so air must not be advanced.
pub fn eyes_in_water(
    state: &MotionState,
    mut block_at: impl FnMut(i32, i32, i32) -> Option<BlockData>,
) -> Option<bool> {
    // The viewer stores its motion position at the foot, unlike native gs,
    // whose entity Y uses a 1.62f offset. Native gs.w() adds 0.12f.
    let y = state.bbox.min[1] + f64::from(1.62_f32) - f64::from(state.y_size) + f64::from(0.12_f32);
    let x = state.position[0].floor();
    let z = state.position[2].floor();
    let cell_y = y.floor();
    if ![x, cell_y, z]
        .into_iter()
        .all(|v| v.is_finite() && v >= f64::from(i32::MIN) && v < f64::from(i32::MAX))
    {
        return None;
    }
    let (x, cell_y, z) = (x as i32, cell_y as i32, z as i32);
    let block = block_at(x, cell_y, z)?;
    if !matches!(block.id, 8 | 9) {
        return Some(false);
    }
    let level = if block.metadata >= 8 {
        0
    } else {
        block.metadata
    };
    let offset = (f32::from(level) + 1.0_f32) / 9.0_f32 - 0.111_111_11_f32;
    let surface = (cell_y + 1) as f32 - offset;
    Some(y < f64::from(surface))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn water(metadata: u8) -> BlockData {
        BlockData {
            id: 9,
            metadata,
            block_light: 0,
            sky_light: 15,
        }
    }

    #[test]
    fn native_eye_query_matrix() {
        for row in include_str!("../../../tests/fixtures/drowning/native_eye.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = row.split('\t').collect();
            let mut state = MotionState::new([-0.2, fields[0].parse().unwrap(), 0.5]);
            state.y_size = fields[1].parse().unwrap();
            let block = BlockData {
                id: fields[2].parse().unwrap(),
                metadata: fields[3].parse().unwrap(),
                block_light: 0,
                sky_light: 0,
            };
            assert_eq!(
                eyes_in_water(&state, |_, _, _| Some(block)),
                Some(fields[4].parse().unwrap()),
                "{row}"
            );
        }
    }

    #[test]
    fn eye_height_and_water_level_determine_submersion() {
        let mut state = MotionState::new([-0.2, 62.0, 0.5]);
        assert_eq!(
            eyes_in_water(&state, |x, y, z| {
                assert_eq!((x, y, z), (-1, 63, 0));
                Some(water(0))
            }),
            Some(true)
        );
        assert_eq!(eyes_in_water(&state, |_, _, _| Some(water(7))), Some(false));
        assert_eq!(eyes_in_water(&state, |_, _, _| Some(water(8))), Some(true));
        state.bbox.min[1] = 62.3;
        assert_eq!(eyes_in_water(&state, |_, _, _| Some(water(0))), Some(true));
        assert_eq!(eyes_in_water(&state, |_, _, _| None), None);
    }
}
