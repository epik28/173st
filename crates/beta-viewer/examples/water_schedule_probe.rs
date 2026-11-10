use std::{collections::BTreeMap, io::Read};

use beta_core::water::{WaterBlock, WaterCell, WaterEvent, WaterPos};
use beta_viewer::water_runtime::{CapturedSeedAction, WaterRuntime};

const MIN_XZ: i32 = -1;
const MAX_XZ: i32 = 1;
const MIN_Y: i32 = 62;
const MAX_Y: i32 = 65;

fn main() {
    let mut input = String::new();
    if let Err(error) = std::io::stdin().read_to_string(&mut input) {
        eprintln!("water_schedule_probe: {error}");
        std::process::exit(2);
    }
    match run_probe(&input) {
        Ok(output) => print!("{output}"),
        Err(error) => {
            eprintln!("water_schedule_probe: {error}");
            std::process::exit(2);
        }
    }
}

fn run_probe(input: &str) -> Result<String, String> {
    let mut output = String::new();
    let mut case_name: Option<String> = None;
    let mut cells = BTreeMap::new();
    let mut action = None;
    for line in input.lines().filter(|line| !line.trim().is_empty()) {
        if line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        match fields.first().copied().unwrap_or_default() {
            "case" => {
                if case_name.is_some() {
                    return Err("nested case".to_owned());
                }
                if fields.len() < 2 {
                    return Err("case requires a name".to_owned());
                }
                case_name = Some(fields[1].to_owned());
                cells = empty_scene();
                action = None;
            }
            "input" => {
                let position = parse_pos(&fields, 1)?;
                let block = WaterBlock {
                    id: parse_field(&fields, 4)?,
                    metadata: parse_field(&fields, 5)?,
                };
                cells.insert(position, WaterCell::Loaded(block));
            }
            "action" => {
                if action.is_some() {
                    return Err("case has multiple actions".to_owned());
                }
                action = Some(parse_action(&fields)?);
            }
            _ => return Err(format!("unknown input record: {}", fields[0])),
        }
        if line.starts_with("action\t") {
            let name = case_name.as_deref().ok_or("action outside case")?;
            let mut runtime = WaterRuntime::default();
            let result = runtime
                .apply_captured_seed(cells.clone(), action.take().ok_or("missing action")?)
                .map_err(|error| format!("{name}: {error}"))?;
            output.push_str(&format!("case\t{name}\n"));
            for event in &result.events {
                match event {
                    WaterEvent::Edit {
                        position,
                        before,
                        after,
                    } => output.push_str(&format!(
                        "edit\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                        position.0,
                        position.1,
                        position.2,
                        before.id,
                        before.metadata,
                        after.id,
                        after.metadata
                    )),
                    WaterEvent::Schedule {
                        position,
                        id,
                        delay,
                    } => output.push_str(&format!(
                        "schedule\t{}\t{}\t{}\t{}\t{}\n",
                        position.0, position.1, position.2, id, delay
                    )),
                    WaterEvent::Neighbor { position, id } => output.push_str(&format!(
                        "neighbor\t{}\t{}\t{}\t{}\n",
                        position.0, position.1, position.2, id
                    )),
                }
            }
            output.push_str("queue_after");
            for tick in runtime.queue() {
                output.push_str(&format!(
                    "\t{}:{}:{}:{}:{}",
                    tick.position.0, tick.position.1, tick.position.2, tick.id, tick.due_time
                ));
            }
            output.push('\n');
            output.push_str("state\t");
            let mut first = true;
            for y in (MIN_Y..=MAX_Y).rev() {
                for z in MIN_XZ..=MAX_XZ {
                    for x in MIN_XZ..=MAX_XZ {
                        if !first {
                            output.push(';');
                        }
                        first = false;
                        let cell = result.cells.get(&(x, y, z)).ok_or("incomplete state")?;
                        match cell {
                            WaterCell::Loaded(block) => output
                                .push_str(&format!("{x},{y},{z}={}:{}", block.id, block.metadata)),
                            WaterCell::Unloaded => return Err("unloaded fixture cell".to_owned()),
                        }
                    }
                }
            }
            output.push(';');
            output.push('\n');
            output.push_str(&format!("#case_end\t{name}\n"));
            case_name = None;
        }
    }
    if case_name.is_some() {
        return Err("unterminated case".to_owned());
    }
    Ok(output)
}

fn empty_scene() -> BTreeMap<WaterPos, WaterCell> {
    let mut cells = BTreeMap::new();
    for y in MIN_Y..=MAX_Y {
        for z in MIN_XZ..=MAX_XZ {
            for x in MIN_XZ..=MAX_XZ {
                cells.insert(
                    (x, y, z),
                    WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
                );
            }
        }
    }
    cells
}

fn parse_action(fields: &[&str]) -> Result<CapturedSeedAction, String> {
    if fields.len() < 7 {
        return Err("action requires operation, xyz, id, metadata".to_owned());
    }
    let pos = parse_pos(fields, 2)?;
    let id: u8 = parse_field(fields, 5)?;
    let metadata: u8 = parse_field(fields, 6)?;
    match fields[1] {
        "place-flow" if id == 8 && metadata == 0 => Ok(CapturedSeedAction::PlaceFlowing(pos)),
        "place-solid-and-notify" if id == 1 && metadata == 0 => {
            Ok(CapturedSeedAction::PlaceAdjacentSolidAndNotify(pos))
        }
        "remove-support-and-notify" if id == 0 && metadata == 0 => {
            Ok(CapturedSeedAction::RemoveSourceSupportAndNotify(pos))
        }
        action => Err(format!(
            "unsupported captured operation/state: {action} {id}:{metadata}"
        )),
    }
}

fn parse_pos(fields: &[&str], start: usize) -> Result<WaterPos, String> {
    Ok((
        parse_field(fields, start)?,
        parse_field(fields, start + 1)?,
        parse_field(fields, start + 2)?,
    ))
}

fn parse_field<T: std::str::FromStr>(fields: &[&str], index: usize) -> Result<T, String> {
    fields
        .get(index)
        .ok_or_else(|| format!("missing field {index}"))?
        .parse()
        .map_err(|_| format!("invalid field {index}"))
}

#[cfg(test)]
mod tests {
    use super::run_probe;

    const INPUT: &str = include_str!("../../../tests/fixtures/water_tick_schedule/input.tsv");
    const NATIVE: &str = include_str!("../../../tests/fixtures/water_tick_schedule/native.tsv");

    #[test]
    fn frozen_seed_operations_match_ordered_events_queue_and_full_state() {
        let actual = run_probe(INPUT).unwrap();
        let expected = relevant_records(NATIVE);
        assert_eq!(actual, expected);
    }

    fn relevant_records(native: &str) -> String {
        let mut result = String::new();
        let mut in_case = false;
        for line in native.lines() {
            if line.starts_with("case\t") {
                in_case = true;
                let name = line.split('\t').nth(1).expect("case has a name");
                result.push_str(&format!("case\t{name}"));
                result.push('\n');
            } else if in_case
                && (line.starts_with("edit\t")
                    || line.starts_with("schedule\t")
                    || line.starts_with("neighbor\t")
                    || line.starts_with("queue_after\t")
                    || line == "queue_after"
                    || line.starts_with("state\t"))
            {
                result.push_str(line);
                result.push('\n');
            } else if line.starts_with("#case_end\t") && in_case {
                result.push_str(line);
                result.push('\n');
                in_case = false;
            }
        }
        result
    }
}
