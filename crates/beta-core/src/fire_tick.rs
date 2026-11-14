//! Native entity-tick fire countdown and lava contact.
//! This is the bounded counter/event portion, without contact detection,
//! cooldown, health, fire block collision, sounds, or particles.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FireTick {
    pub fire: i32,
    /// Ordered damage requests: burning first, lava contact second.
    pub damage: [Option<u8>; 2],
}

/// Apply the native `sn.U()` water, burning and lava branches in order.
#[must_use]
pub fn tick(fire: i32, wet: bool, lava: bool, fire_immune: bool, remote: bool) -> FireTick {
    let mut fire = if wet || remote { 0 } else { fire };
    let mut damage = [None, None];
    if !remote && fire > 0 {
        if fire_immune {
            fire = (fire - 4).max(0);
        } else {
            if fire % 20 == 0 {
                damage[0] = Some(1);
            }
            fire -= 1;
        }
    }
    if lava && !fire_immune {
        damage[1] = Some(4);
        fire = 600;
    }
    // `ls.U()` clears the counter after the base tick for immune or remote entities.
    if fire_immune || remote {
        fire = 0;
    }
    FireTick { fire, damage }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_entity_tick_matrix() {
        for line in include_str!("../../../tests/fixtures/fire_tick/native.tsv")
            .lines()
            .skip(1)
        {
            let f: Vec<_> = line.split('\t').collect();
            let result = tick(
                f[0].parse().unwrap(),
                f[1].parse().unwrap(),
                f[2].parse().unwrap(),
                f[3].parse().unwrap(),
                f[4].parse().unwrap(),
            );
            assert_eq!(result.fire, f[5].parse().unwrap(), "{line}");
            let events: String = result
                .damage
                .into_iter()
                .flatten()
                .map(|v| char::from(b'0' + v))
                .collect();
            assert_eq!(
                if events.is_empty() { "-" } else { &events },
                f[6],
                "{line}"
            );
        }
    }
}
