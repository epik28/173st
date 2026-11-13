//! Native living-entity health fields read from player NBT.
use beta_core::nbt::Tag;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SavedHealth {
    pub health: i32,
    pub hurt_time: i32,
    pub death_time: i32,
    pub attack_time: i32,
}

/// Extract the fields read by native `ls.a(nu)`. A missing Health field on an
/// existing entity means 10, while a new player (no saved entity) starts at 20.
/// Invalid NBT and duplicate keys are rejected; no world data is modified.
#[must_use]
pub fn read(player: Option<&Tag>) -> Option<SavedHealth> {
    let Some(player) = player else {
        return Some(SavedHealth {
            health: 20,
            hurt_time: 0,
            death_time: 0,
            attack_time: 0,
        });
    };
    let Tag::Compound(fields) = player else {
        return None;
    };
    let get = |key: &str, default: i32| -> Option<i32> {
        let mut matching = fields.iter().filter(|(name, _)| name == key);
        let first = matching.next();
        if matching.next().is_some() {
            return None;
        }
        match first {
            None => Some(default),
            Some((_, Tag::Short(value))) => Some(i32::from(*value)),
            Some(_) => None,
        }
    };
    Some(SavedHealth {
        health: get("Health", 10)?,
        hurt_time: get("HurtTime", 0)?,
        death_time: get("DeathTime", 0)?,
        attack_time: get("AttackTime", 0)?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_native_entity_load() {
        for line in include_str!("../../../tests/fixtures/player_health_nbt/native.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<_> = line.split('\t').collect();
            let player = if fields[0] == "missing" {
                Tag::Compound(vec![])
            } else {
                Tag::Compound(vec![
                    ("Health".into(), Tag::Short(fields[0].parse().unwrap())),
                    ("HurtTime".into(), Tag::Short(5)),
                    ("DeathTime".into(), Tag::Short(6)),
                    ("AttackTime".into(), Tag::Short(7)),
                ])
            };
            let state = read(Some(&player)).unwrap();
            assert_eq!(
                [
                    state.health,
                    state.hurt_time,
                    state.death_time,
                    state.attack_time
                ],
                [
                    fields[1].parse().unwrap(),
                    fields[2].parse().unwrap(),
                    fields[3].parse().unwrap(),
                    fields[4].parse().unwrap()
                ],
                "{line}"
            );
        }
        assert_eq!(read(None).unwrap().health, 20);
        assert!(read(Some(&Tag::Compound(vec![("Health".into(), Tag::Int(20))]))).is_none());
    }
}
