//! Bounded player fire inputs from saved state and native lava contact.
use beta_core::{chunk::BlockData, motion::MotionState, nbt::Tag};

pub fn saved_fire(player: Option<&Tag>) -> Option<i32> {
    let Some(player) = player else { return Some(0) };
    let Tag::Compound(fields) = player else {
        return None;
    };
    let mut values = fields.iter().filter(|(name, _)| name == "Fire");
    let value = values.next();
    if values.next().is_some() {
        return None;
    }
    match value {
        None => Some(0),
        Some((_, Tag::Short(v))) => Some(i32::from(*v)),
        _ => None,
    }
}

/// Native sn.ah contracts the box by 0.1f horizontally and 0.4f vertically,
/// then fd.a checks the material of every touched cell (without a fluid-level test).
/// Missing cells return None so the caller can avoid inventing a contact state.
pub fn lava_contact(
    state: &MotionState,
    mut block: impl FnMut(i32, i32, i32) -> Option<BlockData>,
) -> Option<bool> {
    let h = f64::from(0.1_f32);
    let v = f64::from(0.4_f32);
    let lo = [
        state.bbox.min[0] + h,
        state.bbox.min[1] + v,
        state.bbox.min[2] + h,
    ];
    let hi = [
        state.bbox.max[0] - h,
        state.bbox.max[1] - v,
        state.bbox.max[2] - h,
    ];
    let floor = |x: f64| -> Option<i32> {
        let v = x.floor();
        if v.is_finite() && v >= i32::MIN as f64 && v < i32::MAX as f64 {
            Some(v as i32)
        } else {
            None
        }
    };
    let (x0, y0, z0) = (floor(lo[0])?, floor(lo[1])?, floor(lo[2])?);
    let (x1, y1, z1) = (
        floor(hi[0] + 1.0)?,
        floor(hi[1] + 1.0)?,
        floor(hi[2] + 1.0)?,
    );
    if x1 - x0 > 4 || y1 - y0 > 4 || z1 - z0 > 4 {
        return None;
    }
    let mut missing = false;
    for x in x0..x1 {
        for y in y0..y1 {
            for z in z0..z1 {
                match block(x, y, z) {
                    Some(b) if matches!(b.id, 10 | 11) => return Some(true),
                    None => missing = true,
                    _ => {}
                }
            }
        }
    }
    if missing { None } else { Some(false) }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_lava_contact_matrix() {
        for row in include_str!("../../../tests/fixtures/fire_contact/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = row.split('\t').collect();
            let state = MotionState::new([
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
            let result = lava_contact(&state, |x, y, z| {
                Some(BlockData {
                    id: if (x, y, z) == pos { id } else { 0 },
                    metadata: 0,
                    block_light: 0,
                    sky_light: 0,
                })
            });
            assert_eq!(result, Some(f[7].parse().unwrap()), "{row}");
        }
    }
    #[test]
    fn fire_nbt_is_short_and_unknown_contact_stays_unknown() {
        assert_eq!(saved_fire(None), Some(0));
        assert_eq!(
            saved_fire(Some(&Tag::Compound(vec![("Fire".into(), Tag::Short(-20))]))),
            Some(-20)
        );
        assert_eq!(
            saved_fire(Some(&Tag::Compound(vec![("Fire".into(), Tag::Int(20))]))),
            None
        );
        assert_eq!(
            lava_contact(&MotionState::new([0.5, 0.0, 0.5]), |_, _, _| None),
            None
        );
    }
}
