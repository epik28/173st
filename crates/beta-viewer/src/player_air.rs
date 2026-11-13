//! Read the player's saved `Air` field without modifying the world.

use beta_core::nbt::Tag;

/// Extract native short air. A new player starts at 300; native NBT's missing
/// short getter returns 0 for an existing player compound. Invalid types and
/// duplicate fields are rejected rather than silently changing saved state.
#[must_use]
pub fn saved_air(player: Option<&Tag>) -> Option<i32> {
    let Some(player) = player else {
        return Some(300);
    };
    let Tag::Compound(fields) = player else {
        return None;
    };
    let mut matches = fields.iter().filter(|(name, _)| name == "Air");
    let value = matches.next();
    if matches.next().is_some() {
        return None;
    }
    match value {
        None => Some(0),
        Some((_, Tag::Short(air))) => Some(i32::from(*air)),
        Some(_) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_native_nbt_short_getter() {
        for row in include_str!("../../../tests/fixtures/player_air/native.tsv")
            .lines()
            .skip(1)
        {
            let (case, value) = row.split_once('\t').unwrap();
            let player = match case {
                "missing" => Tag::Compound(vec![]),
                "wrong_type" => Tag::Compound(vec![("Air".into(), Tag::Int(300))]),
                _ => Tag::Compound(vec![("Air".into(), Tag::Short(case.parse().unwrap()))]),
            };
            let expected = if value == "rejected" {
                None
            } else {
                Some(value.parse().unwrap())
            };
            assert_eq!(saved_air(Some(&player)), expected, "{case}");
        }
        assert_eq!(saved_air(None), Some(300));
        assert_eq!(
            saved_air(Some(&Tag::Compound(vec![
                ("Air".into(), Tag::Short(1)),
                ("Air".into(), Tag::Short(2))
            ]))),
            None
        );
    }
}
