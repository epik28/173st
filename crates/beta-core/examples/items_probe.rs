//! Bounded atomic TSV adapter for the native-confirmed inventory slice.

use std::{
    collections::HashSet,
    io::{self, Read, Write},
};

use beta_core::items::{Inventory, ItemStack, MAIN_SLOT_COUNT};

const MAX_INPUT_BYTES: usize = 1_000_000;
const MAX_SCENARIOS: usize = 256;

#[derive(Debug)]
enum Scenario {
    SplitRemoveReplace { id: u16, count: u8, split: u8 },
    Merge { id: u16, have: u8, incoming: u8 },
    Overflow { full_id: u16, incoming_id: u16 },
    SmallerLimit { id: u16, have: u8, incoming: u8 },
    NonStackable { id: u16 },
    HotbarBounds,
}

fn canonical_number<T>(value: &str, label: &str) -> Result<T, String>
where
    T: std::str::FromStr + ToString,
{
    let parsed = value
        .parse::<T>()
        .map_err(|_| format!("{label} is not a valid decimal number"))?;
    if parsed.to_string() != value {
        return Err(format!("{label} must use canonical decimal notation"));
    }
    Ok(parsed)
}

fn make_stack(id: u16, count: u8) -> Result<ItemStack, String> {
    ItemStack::new(id, count, 0).map_err(|error| error.to_string())
}

fn parse_scenarios(input: &[u8]) -> Result<Vec<Scenario>, String> {
    if input.is_empty() || input.len() > MAX_INPUT_BYTES {
        return Err("input is empty or exceeds byte limit".to_owned());
    }
    let text = std::str::from_utf8(input).map_err(|_| "input must be UTF-8".to_owned())?;
    if !text.ends_with('\n') || text.contains('\r') {
        return Err("input rows require canonical LF termination".to_owned());
    }

    let mut scenarios = Vec::new();
    let mut ids = HashSet::new();
    for (line_number, line) in text[..text.len() - 1].split('\n').enumerate() {
        if line.starts_with('#') {
            continue;
        }
        if line.is_empty() {
            return Err(format!("line {} is empty", line_number + 1));
        }
        let fields: Vec<_> = line.split('\t').collect();
        if fields.iter().any(|field| field.is_empty()) {
            return Err(format!("line {} contains an empty field", line_number + 1));
        }
        let name = fields[0];
        if !name.bytes().all(|byte| {
            byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_' || byte == b'-'
        }) {
            return Err(format!(
                "line {} has an invalid scenario ID",
                line_number + 1
            ));
        }
        if !ids.insert(name.to_owned()) {
            return Err(format!("duplicate scenario {name}"));
        }
        let number = |index: usize, label: &str| -> Result<&str, String> {
            fields
                .get(index)
                .copied()
                .ok_or_else(|| format!("{name}: missing {label}"))
        };
        let scenario = match name {
            "split_remove_replace" if fields.len() == 4 => {
                let id = canonical_number(number(1, "item ID")?, "item ID")?;
                let count = canonical_number(number(2, "count")?, "count")?;
                let split = canonical_number(number(3, "split count")?, "split count")?;
                let stack = make_stack(id, count)?;
                if split == 0 || split > count {
                    return Err(format!("{name}: split count must be in 1..=initial count"));
                }
                if id == 3 {
                    return Err(format!(
                        "{name}: replacement item must differ from initial item"
                    ));
                }
                let _ = stack;
                Scenario::SplitRemoveReplace { id, count, split }
            }
            "merge" if fields.len() == 4 => {
                let id = canonical_number(number(1, "item ID")?, "item ID")?;
                let have = canonical_number(number(2, "existing count")?, "existing count")?;
                let incoming = canonical_number(number(3, "incoming count")?, "incoming count")?;
                make_stack(id, have)?;
                make_stack(id, incoming)?;
                Scenario::Merge { id, have, incoming }
            }
            "overflow" if fields.len() == 3 => {
                let full_id = canonical_number(number(1, "full item ID")?, "full item ID")?;
                let incoming_id =
                    canonical_number(number(2, "incoming item ID")?, "incoming item ID")?;
                let full = make_stack(full_id, 64)?;
                let incoming = make_stack(incoming_id, 1)?;
                if full_id == incoming_id {
                    return Err(format!(
                        "{name}: incoming item must be incompatible with full slots"
                    ));
                }
                let _ = (full, incoming);
                Scenario::Overflow {
                    full_id,
                    incoming_id,
                }
            }
            "smaller_limit" if fields.len() == 4 => {
                let id = canonical_number(number(1, "item ID")?, "item ID")?;
                let have = canonical_number(number(2, "existing count")?, "existing count")?;
                let incoming = canonical_number(number(3, "incoming count")?, "incoming count")?;
                let stack = make_stack(id, have)?;
                make_stack(id, incoming)?;
                if stack.max_stack() == 64 {
                    return Err(format!(
                        "{name}: item must have a captured smaller stack limit"
                    ));
                }
                Scenario::SmallerLimit { id, have, incoming }
            }
            "non_stackable" if fields.len() == 2 => {
                let id = canonical_number(number(1, "item ID")?, "item ID")?;
                let stack = make_stack(id, 1)?;
                if stack.max_stack() != 1 {
                    return Err(format!("{name}: item must be captured as non-stackable"));
                }
                Scenario::NonStackable { id }
            }
            "hotbar_bounds" if fields.len() == 1 => Scenario::HotbarBounds,
            _ => return Err(format!("{name}: unknown scenario or wrong field count")),
        };
        scenarios.push(scenario);
        if scenarios.len() > MAX_SCENARIOS {
            return Err("scenario count exceeds limit".to_owned());
        }
    }
    if scenarios.is_empty() {
        return Err("input has no scenarios".to_owned());
    }
    Ok(scenarios)
}

fn stack_token(stack: Option<ItemStack>) -> String {
    match stack {
        Some(item) => format!(
            "{},{},{},{}",
            item.id(),
            item.count(),
            item.damage(),
            item.max_stack()
        ),
        None => "-".to_owned(),
    }
}

fn stack_token_with_count(id: u16, count: u8) -> Result<String, String> {
    if count == 0 {
        let exemplar = make_stack(id, 1)?;
        Ok(format!("{id},0,0,{}", exemplar.max_stack()))
    } else {
        Ok(stack_token(Some(make_stack(id, count)?)))
    }
}

fn emit_state(
    output: &mut String,
    scenario: &str,
    step: &str,
    result: &str,
    selected_raw: Option<i32>,
    inventory: &Inventory,
) {
    let selected = selected_raw.unwrap_or_else(|| i32::from(inventory.selected_hotbar()));
    let selected_stack = match selected_raw {
        Some(index) => inventory.selected_stack_at(index),
        None => inventory.selected_stack(),
    };
    let slots = inventory
        .slots()
        .iter()
        .copied()
        .map(stack_token)
        .collect::<Vec<_>>()
        .join("|");
    debug_assert_eq!(inventory.slots().len(), MAIN_SLOT_COUNT);
    use std::fmt::Write as _;
    writeln!(
        output,
        "{scenario}\t{step}\t{result}\t{selected}\t{}\t{slots}",
        stack_token(selected_stack)
    )
    .expect("writing to String cannot fail");
}

fn run_scenario(output: &mut String, scenario: &Scenario) -> Result<(), String> {
    match *scenario {
        Scenario::SplitRemoveReplace { id, count, split } => {
            let mut inventory = Inventory::new();
            emit_state(
                output,
                "split_remove_replace",
                "empty",
                "none",
                None,
                &inventory,
            );
            inventory
                .set_slot(0, Some(make_stack(id, count)?))
                .map_err(|error| error.to_string())?;
            emit_state(
                output,
                "split_remove_replace",
                "set",
                "slot=0",
                None,
                &inventory,
            );
            let part = inventory
                .split_slot(0, split)
                .map_err(|error| error.to_string())?;
            emit_state(
                output,
                "split_remove_replace",
                "split",
                &format!("returned={}", stack_token(Some(part))),
                None,
                &inventory,
            );
            let removed = inventory.remove_one_by_id(id);
            emit_state(
                output,
                "split_remove_replace",
                "remove_one_by_id",
                &format!("returned={removed};id={id}"),
                None,
                &inventory,
            );
            inventory
                .set_slot(0, Some(make_stack(3, 2)?))
                .map_err(|error| error.to_string())?;
            emit_state(
                output,
                "split_remove_replace",
                "replace",
                "slot=0",
                None,
                &inventory,
            );
        }
        Scenario::Merge { id, have, incoming } => {
            let mut inventory = Inventory::new();
            inventory
                .set_slot(0, Some(make_stack(id, have)?))
                .map_err(|error| error.to_string())?;
            emit_state(
                output,
                "merge",
                "before",
                &format!("incoming={}", stack_token(Some(make_stack(id, incoming)?))),
                None,
                &inventory,
            );
            let mut remaining = Some(make_stack(id, incoming)?);
            let accepted = inventory
                .insert(&mut remaining)
                .map_err(|error| error.to_string())?;
            let count = remaining.map_or(0, ItemStack::count);
            let token = stack_token_with_count(id, count)?;
            emit_state(
                output,
                "merge",
                "after",
                &format!("accepted={accepted};remaining={token}"),
                None,
                &inventory,
            );
        }
        Scenario::Overflow {
            full_id,
            incoming_id,
        } => {
            let mut inventory = Inventory::new();
            for slot in 0..MAIN_SLOT_COUNT {
                inventory
                    .set_slot(slot, Some(make_stack(full_id, 64)?))
                    .map_err(|error| error.to_string())?;
            }
            let item = make_stack(incoming_id, 1)?;
            emit_state(
                output,
                "overflow",
                "before",
                &format!("incoming={}", stack_token(Some(item))),
                None,
                &inventory,
            );
            let mut remaining = Some(item);
            let accepted = inventory
                .insert(&mut remaining)
                .map_err(|error| error.to_string())?;
            let count = remaining.map_or(0, ItemStack::count);
            let token = stack_token_with_count(incoming_id, count)?;
            emit_state(
                output,
                "overflow",
                "after",
                &format!("accepted={accepted};remaining={token}"),
                None,
                &inventory,
            );
        }
        Scenario::SmallerLimit { id, have, incoming } => {
            let mut inventory = Inventory::new();
            inventory
                .set_slot(0, Some(make_stack(id, have)?))
                .map_err(|error| error.to_string())?;
            let item = make_stack(id, incoming)?;
            emit_state(
                output,
                "smaller_limit",
                "before",
                &format!(
                    "max={};incoming={}",
                    item.max_stack(),
                    stack_token(Some(item))
                ),
                None,
                &inventory,
            );
            let mut remaining = Some(item);
            let accepted = inventory
                .insert(&mut remaining)
                .map_err(|error| error.to_string())?;
            let count = remaining.map_or(0, ItemStack::count);
            let token = stack_token_with_count(id, count)?;
            let max = ItemStack::new(id, 1, 0)
                .map_err(|error| error.to_string())?
                .max_stack();
            emit_state(
                output,
                "smaller_limit",
                "after",
                &format!("accepted={accepted};max={max};remaining={token}"),
                None,
                &inventory,
            );
        }
        Scenario::NonStackable { id } => {
            let mut inventory = Inventory::new();
            inventory
                .set_slot(0, Some(make_stack(id, 1)?))
                .map_err(|error| error.to_string())?;
            let item = make_stack(id, 1)?;
            emit_state(
                output,
                "non_stackable",
                "before",
                &format!("max={}", item.max_stack()),
                None,
                &inventory,
            );
            let mut remaining = Some(item);
            let accepted = inventory
                .insert(&mut remaining)
                .map_err(|error| error.to_string())?;
            let count = remaining.map_or(0, ItemStack::count);
            let token = stack_token_with_count(id, count)?;
            emit_state(
                output,
                "non_stackable",
                "after",
                &format!("accepted={accepted};remaining={token}"),
                None,
                &inventory,
            );
        }
        Scenario::HotbarBounds => {
            let mut inventory = Inventory::new();
            inventory
                .set_slot(0, Some(make_stack(1, 2)?))
                .map_err(|error| error.to_string())?;
            inventory
                .set_slot(8, Some(make_stack(3, 4)?))
                .map_err(|error| error.to_string())?;
            emit_state(output, "hotbar_bounds", "initial", "", None, &inventory);
            inventory.cycle_hotbar(1);
            emit_state(
                output,
                "hotbar_bounds",
                "cycle_positive",
                "",
                None,
                &inventory,
            );
            inventory.cycle_hotbar(-1);
            emit_state(
                output,
                "hotbar_bounds",
                "cycle_negative",
                "",
                None,
                &inventory,
            );
            inventory.cycle_hotbar(9);
            emit_state(output, "hotbar_bounds", "cycle_wrap", "", None, &inventory);
            let before_invalid_lookup = inventory.clone();
            emit_state(
                output,
                "hotbar_bounds",
                "index_minus_one",
                "",
                Some(-1),
                &inventory,
            );
            emit_state(
                output,
                "hotbar_bounds",
                "index_nine",
                "",
                Some(9),
                &inventory,
            );
            debug_assert_eq!(inventory, before_invalid_lookup);
            inventory.select_item_id(3);
            emit_state(output, "hotbar_bounds", "select_id_3", "", None, &inventory);
            inventory.select_item_id(999);
            emit_state(
                output,
                "hotbar_bounds",
                "select_missing_id",
                "",
                None,
                &inventory,
            );
        }
    }
    Ok(())
}

fn execute(input: &[u8], stdout: &mut impl Write) -> Result<(), String> {
    let scenarios = parse_scenarios(input)?;
    let mut staged = String::new();
    for scenario in &scenarios {
        run_scenario(&mut staged, scenario)?;
    }
    stdout
        .write_all(staged.as_bytes())
        .map_err(|error| format!("write stdout: {error}"))
}

fn run() -> Result<(), String> {
    let mut input = Vec::new();
    io::stdin()
        .take((MAX_INPUT_BYTES + 1) as u64)
        .read_to_end(&mut input)
        .map_err(|error| format!("read stdin: {error}"))?;
    if input.len() > MAX_INPUT_BYTES {
        return Err("input exceeds byte limit".to_owned());
    }
    execute(&input, &mut io::stdout().lock())
}

fn main() {
    if let Err(error) = run() {
        let _ = writeln!(io::stderr().lock(), "items_probe: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt::Write as _;

    const INPUT: &str = include_str!("../../../tests/fixtures/items/input.tsv");
    const NATIVE: &str = include_str!("../../../tests/fixtures/items/native.tsv");

    fn normalize_native_stack(token: &str) -> String {
        if token == "-" {
            return token.to_owned();
        }
        let fields: Vec<_> = token.split(',').collect();
        assert_eq!(fields.len(), 5);
        fields[..4].join(",")
    }

    fn normalize_native_result(result: &str) -> String {
        if result.is_empty() || result == "none" {
            return result.to_owned();
        }
        result
            .split(';')
            .map(|part| {
                let (key, value) = part.split_once('=').unwrap();
                let normalized = if value.contains(',') {
                    normalize_native_stack(value)
                } else {
                    value.to_owned()
                };
                format!("{key}={normalized}")
            })
            .collect::<Vec<_>>()
            .join(";")
    }

    fn normalized_fixture() -> String {
        let mut output = String::new();
        for line in NATIVE.lines() {
            let fields: Vec<_> = line.split('\t').collect();
            assert_eq!(fields.len(), 6);
            let slots = fields[5]
                .split('|')
                .map(normalize_native_stack)
                .collect::<Vec<_>>()
                .join("|");
            writeln!(
                output,
                "{}\t{}\t{}\t{}\t{}\t{}",
                fields[0],
                fields[1],
                normalize_native_result(fields[2]),
                fields[3],
                normalize_native_stack(fields[4]),
                slots
            )
            .unwrap();
        }
        output
    }

    #[test]
    fn all_21_ordered_native_states_match_exactly() {
        let mut actual = Vec::new();
        execute(INPUT.as_bytes(), &mut actual).unwrap();
        assert_eq!(String::from_utf8(actual).unwrap(), normalized_fixture());
    }

    #[test]
    fn invalid_hotbar_observations_keep_raw_index_without_mutating_inventory() {
        let scenarios = parse_scenarios(b"hotbar_bounds\n").unwrap();
        let mut output = String::new();
        run_scenario(&mut output, &scenarios[0]).unwrap();
        let rows: Vec<_> = output.lines().collect();
        assert!(rows[4].starts_with("hotbar_bounds\tindex_minus_one\t\t-1\t-\t"));
        assert!(rows[5].starts_with("hotbar_bounds\tindex_nine\t\t9\t-\t"));
        let expected_slots = rows[3].split('\t').nth(5).unwrap();
        assert_eq!(rows[4].split('\t').nth(5).unwrap(), expected_slots);
        assert_eq!(rows[5].split('\t').nth(5).unwrap(), expected_slots);
    }

    #[test]
    fn malformed_late_duplicate_unsupported_and_oversized_inputs_are_atomic() {
        let bad_inputs: Vec<Vec<u8>> = vec![
            b"hotbar_bounds\nsplit_remove_replace\t1\t10\t0\n".to_vec(),
            b"hotbar_bounds\nhotbar_bounds\n".to_vec(),
            b"non_stackable\t999\n".to_vec(),
            b"split_remove_replace\t1\t10\t11\n".to_vec(),
            b"hotbar_bounds\r\n".to_vec(),
            format!("hotbar_bounds\n{}", "x".repeat(MAX_INPUT_BYTES)).into_bytes(),
        ];
        for input in bad_inputs {
            let mut output = Vec::new();
            assert!(execute(&input, &mut output).is_err());
            assert!(output.is_empty());
        }
    }

    #[test]
    fn rejects_wrong_field_counts_noncanonical_values_and_scenario_overflow() {
        for input in [
            "merge\t1\t50\n",
            "merge\t01\t50\t20\n",
            "overflow\t1\t1\n",
            "smaller_limit\t1\t50\t20\n",
            "non_stackable\t332\n",
        ] {
            assert!(
                parse_scenarios(input.as_bytes()).is_err(),
                "accepted {input:?}"
            );
        }
        let too_many = (0..=MAX_SCENARIOS)
            .map(|_| "hotbar_bounds\n")
            .collect::<String>();
        assert!(parse_scenarios(too_many.as_bytes()).is_err());
    }
}
