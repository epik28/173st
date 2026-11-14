//! Native moveEntity block-collision callbacks for cactus damage.
use beta_core::{chunk::BlockData, motion::MotionState};

/// Count cactus callbacks in native x/y/z order; missing cells abort the query.
/// Each callback requests one damage; the health cooldown decides actual damage.
pub fn cactus_contacts(
    state: &MotionState,
    mut block: impl FnMut(i32, i32, i32) -> Option<BlockData>,
) -> Option<u8> {
    let floor = |x: f64| -> Option<i32> {
        let x = x.floor();
        if x.is_finite() && x >= i32::MIN as f64 && x < i32::MAX as f64 {
            Some(x as i32)
        } else {
            None
        }
    };
    let lo = [
        floor(state.bbox.min[0] + 0.001)?,
        floor(state.bbox.min[1] + 0.001)?,
        floor(state.bbox.min[2] + 0.001)?,
    ];
    let hi = [
        floor(state.bbox.max[0] - 0.001)?,
        floor(state.bbox.max[1] - 0.001)?,
        floor(state.bbox.max[2] - 0.001)?,
    ];
    if (0..3).any(|i| i64::from(hi[i]) - i64::from(lo[i]) > 4 || hi[i] < lo[i]) {
        return None;
    }
    let mut count = 0;
    for x in lo[0]..=hi[0] {
        for y in lo[1]..=hi[1] {
            for z in lo[2]..=hi[2] {
                if block(x, y, z)?.id == 81 {
                    count += 1;
                }
            }
        }
    }
    Some(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_cactus_move_contact() {
        for row in include_str!("../../../tests/fixtures/cactus_contact/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = row.split('\t').collect();
            let s = MotionState::new([
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
            ]);
            let pos = (
                f[3].parse().unwrap(),
                f[4].parse().unwrap(),
                f[5].parse().unwrap(),
            );
            let id = f[6].parse().unwrap();
            let result = cactus_contacts(&s, |x, y, z| {
                Some(BlockData {
                    id: if (x, y, z) == pos { id } else { 0 },
                    metadata: 0,
                    block_light: 0,
                    sky_light: 0,
                })
            });
            let expected = if f[7] == "-" { 0 } else { f[7].len() as u8 };
            assert_eq!(result, Some(expected), "{row}");
        }
        assert_eq!(
            cactus_contacts(&MotionState::new([0.5, 0., 0.5]), |_, _, _| None),
            None
        );
    }
}
