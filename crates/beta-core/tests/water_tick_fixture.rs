use std::collections::BTreeMap;

use beta_core::water::{
    WaterBlock, WaterCell, WaterEvent, WaterPos, WaterTick, WaterTickInput, transition_water_ticks,
};

#[allow(dead_code)]
#[path = "../examples/water_tick_probe.rs"]
mod water_tick_probe;

const NATIVE: &str = include_str!("../../../tests/fixtures/water_tick/native.tsv");
const NATIVE_SOURCES: &str = include_str!("../../../tests/fixtures/water_tick_sources/native.tsv");

#[derive(Default)]
struct NativeStep {
    world_time: i64,
    queue_before: Vec<WaterTick>,
    queue_after: Vec<WaterTick>,
    state: BTreeMap<WaterPos, WaterBlock>,
    events: Vec<WaterEvent>,
    event_lines: Vec<String>,
}

#[derive(Default)]
struct NativeCase {
    name: String,
    initial: BTreeMap<WaterPos, WaterBlock>,
    steps: Vec<NativeStep>,
    pending_time: i64,
    pending_queue: Vec<WaterTick>,
    step: Option<NativeStep>,
}

impl NativeCase {
    fn finish_step(&mut self) {
        if let Some(step) = self.step.take() {
            self.steps.push(step);
        }
    }
}

fn parse_queue(value: &str) -> Vec<WaterTick> {
    if value.is_empty() {
        return Vec::new();
    }
    value
        .split(',')
        .map(|token| {
            let parts: Vec<_> = token.split(':').collect();
            assert_eq!(parts.len(), 5, "malformed fixture queue item {token}");
            WaterTick {
                position: (
                    parts[0].parse().unwrap(),
                    parts[1].parse().unwrap(),
                    parts[2].parse().unwrap(),
                ),
                id: parts[3].parse().unwrap(),
                due_time: parts[4].parse().unwrap(),
            }
        })
        .collect()
}

fn parse_events(case: &mut NativeCase, fields: &[&str]) {
    let step = case.step.as_mut().expect("event outside a native step");
    step.event_lines.push(fields.join("\t"));
    match fields[0] {
        "edit" => step.events.push(WaterEvent::Edit {
            position: (
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
            ),
            before: WaterBlock {
                id: fields[4].parse().unwrap(),
                metadata: fields[5].parse().unwrap(),
            },
            after: WaterBlock {
                id: fields[6].parse().unwrap(),
                metadata: fields[7].parse().unwrap(),
            },
        }),
        "schedule" => step.events.push(WaterEvent::Schedule {
            position: (
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
            ),
            id: fields[4].parse().unwrap(),
            delay: fields[5].parse().unwrap(),
        }),
        "neighbor" => step.events.push(WaterEvent::Neighbor {
            position: (
                fields[1].parse().unwrap(),
                fields[2].parse().unwrap(),
                fields[3].parse().unwrap(),
            ),
            id: fields[4].parse().unwrap(),
        }),
        other => panic!("unexpected native event {other}"),
    }
}

fn parse_native(native: &str) -> Vec<NativeCase> {
    let mut cases = Vec::new();
    let mut case: Option<NativeCase> = None;
    for line in native.lines() {
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "case" => {
                if let Some(mut previous) = case.take() {
                    previous.finish_step();
                    cases.push(previous);
                }
                case = Some(NativeCase {
                    name: fields[1].to_owned(),
                    ..NativeCase::default()
                });
            }
            "setup_edit" => {
                let case = case.as_mut().unwrap();
                case.initial.insert(
                    (
                        fields[1].parse().unwrap(),
                        fields[2].parse().unwrap(),
                        fields[3].parse().unwrap(),
                    ),
                    WaterBlock {
                        id: fields[6].parse().unwrap(),
                        metadata: fields[7].parse().unwrap(),
                    },
                );
            }
            "queue_before" => {
                let case = case.as_mut().unwrap();
                case.finish_step();
                case.pending_queue = parse_queue(fields[1]);
                case.pending_time = fields[2]
                    .strip_prefix("world_time=")
                    .unwrap()
                    .parse()
                    .unwrap();
            }
            "step" => {
                let case = case.as_mut().unwrap();
                let queue_after = fields[3]
                    .strip_prefix("queue_after=")
                    .expect("malformed step record");
                case.step = Some(NativeStep {
                    world_time: case.pending_time,
                    queue_before: std::mem::take(&mut case.pending_queue),
                    queue_after: parse_queue(queue_after),
                    ..NativeStep::default()
                });
            }
            "state" => {
                let state = &mut case.as_mut().unwrap().step.as_mut().unwrap().state;
                for item in fields[1].split(';').filter(|value| !value.is_empty()) {
                    let (coordinate, value) = item.split_once('=').unwrap();
                    let mut xyz = coordinate.split(',').map(|part| part.parse().unwrap());
                    let position = (
                        xyz.next().unwrap(),
                        xyz.next().unwrap(),
                        xyz.next().unwrap(),
                    );
                    let (id, metadata) = value.split_once(':').unwrap();
                    state.insert(
                        position,
                        WaterBlock {
                            id: id.parse().unwrap(),
                            metadata: metadata.parse().unwrap(),
                        },
                    );
                }
            }
            "edit" | "schedule" | "neighbor" => {
                parse_events(case.as_mut().unwrap(), &fields);
            }
            "#case_end" => {
                let mut finished = case.take().unwrap();
                finished.finish_step();
                cases.push(finished);
            }
            _ => {}
        }
    }
    if let Some(mut finished) = case {
        finished.finish_step();
        cases.push(finished);
    }
    cases
}

fn full_cells(initial: &BTreeMap<WaterPos, WaterBlock>) -> BTreeMap<WaterPos, WaterCell> {
    let mut cells = BTreeMap::new();
    for y in 62..=65 {
        for z in -2..=2 {
            for x in -2..=2 {
                let position = (x, y, z);
                cells.insert(
                    position,
                    WaterCell::Loaded(
                        *initial
                            .get(&position)
                            .unwrap_or(&WaterBlock { id: 0, metadata: 0 }),
                    ),
                );
            }
        }
    }
    cells
}

fn assert_native_fixture(native: &str, expected_cases: usize) {
    let cases = parse_native(native);
    assert_eq!(cases.len(), expected_cases);
    for case in cases {
        let mut cells = full_cells(&case.initial);
        for (index, expected) in case.steps.iter().enumerate() {
            assert_eq!(expected.state.len(), 100, "{} step {index}", case.name);
            let probe_input = probe_input(&cells, &expected.queue_before, expected.world_time);
            let probe_output = water_tick_probe::run_probe(&probe_input)
                .unwrap_or_else(|error| panic!("{} step {index}: {error}", case.name));
            let expected_probe = expected_probe_lines(expected);
            assert_eq!(
                probe_output.lines().collect::<Vec<_>>(),
                expected_probe,
                "{} probe step {index}",
                case.name
            );
            let actual = transition_water_ticks(WaterTickInput {
                world_time: expected.world_time,
                cells,
                queue: expected.queue_before.clone(),
                max_ticks: 64,
            })
            .unwrap_or_else(|error| panic!("{} step {index}: {error:?}", case.name));
            assert_eq!(actual.events, expected.events, "{} step {index}", case.name);
            assert_eq!(
                actual.queue_after, expected.queue_after,
                "{} step {index}",
                case.name
            );
            let actual_state: BTreeMap<_, _> = actual
                .cells
                .iter()
                .map(|(position, cell)| match cell {
                    WaterCell::Loaded(block) => (*position, *block),
                    WaterCell::Unloaded => panic!("fixture scene unexpectedly unloaded"),
                })
                .collect();
            assert_eq!(actual_state, expected.state, "{} step {index}", case.name);
            cells = actual.cells;
        }
    }
}

fn queue_text(queue: &[WaterTick]) -> String {
    queue
        .iter()
        .map(|tick| {
            format!(
                "{}:{}:{}:{}:{}",
                tick.position.0, tick.position.1, tick.position.2, tick.id, tick.due_time
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn probe_input(
    cells: &BTreeMap<WaterPos, WaterCell>,
    queue: &[WaterTick],
    world_time: i64,
) -> String {
    let mut input = String::new();
    for y in (62..=65).rev() {
        for z in -2..=2 {
            for x in -2..=2 {
                let position = (x, y, z);
                match cells.get(&position).unwrap() {
                    WaterCell::Loaded(block) => input.push_str(&format!(
                        "cell\t{x}\t{y}\t{z}\t{}\t{}\n",
                        block.id, block.metadata
                    )),
                    WaterCell::Unloaded => panic!("captured probe inputs are fully loaded"),
                }
            }
        }
    }
    for tick in queue {
        input.push_str(&format!(
            "queue\t{}\t{}\t{}\t{}\t{}\n",
            tick.position.0, tick.position.1, tick.position.2, tick.id, tick.due_time
        ));
    }
    input.push_str(&format!("run\t{world_time}\t64\n"));
    input
}

fn expected_probe_lines(step: &NativeStep) -> Vec<String> {
    let processed = step
        .queue_before
        .iter()
        .filter(|tick| tick.due_time <= step.world_time)
        .count();
    let mut lines = vec![format!("result\tprocessed_ticks={processed}")];
    lines.extend(step.event_lines.iter().cloned());
    lines.push(format!("queue_after\t{}", queue_text(&step.queue_after)));
    for y in (62..=65).rev() {
        for z in -2..=2 {
            for x in -2..=2 {
                let block = step.state[&(x, y, z)];
                lines.push(format!(
                    "state\t{x}\t{y}\t{z}\t{}\t{}",
                    block.id, block.metadata
                ));
            }
        }
    }
    lines
}

#[test]
fn native_water_tick_corpus_matches_ordered_transcripts_and_state() {
    assert_native_fixture(NATIVE, 9);
}

#[test]
fn native_source_formation_cases_match_ordered_transcripts_and_state() {
    assert_native_fixture(NATIVE_SOURCES, 4);
}

#[test]
fn due_tick_limit_failure_is_atomic_and_unloaded_cells_are_explicit() {
    let mut cells = full_cells(&BTreeMap::from([(
        (0, 64, 0),
        WaterBlock { id: 8, metadata: 0 },
    )]));
    let queue = vec![WaterTick {
        position: (0, 64, 0),
        id: 8,
        due_time: 1,
    }];
    let before = cells.clone();
    let limit_error = transition_water_ticks(WaterTickInput {
        world_time: 1,
        cells: cells.clone(),
        queue: queue.clone(),
        max_ticks: 0,
    });
    assert!(limit_error.is_err());
    assert_eq!(cells, before);

    let second_due = vec![
        WaterTick {
            position: (0, 64, 0),
            id: 8,
            due_time: 1,
        },
        WaterTick {
            position: (0, 64, 0),
            id: 8,
            due_time: 1,
        },
    ];
    assert!(matches!(
        transition_water_ticks(WaterTickInput {
            world_time: 1,
            cells: cells.clone(),
            queue: second_due,
            max_ticks: 1,
        }),
        Err(beta_core::water::WaterError::TickLimitExceeded { limit: 1 })
    ));
    assert_eq!(cells, before);

    cells.insert((0, 63, 0), WaterCell::Unloaded);
    assert!(matches!(
        transition_water_ticks(WaterTickInput {
            world_time: 1,
            cells,
            queue,
            max_ticks: 1,
        }),
        Err(beta_core::water::WaterError::UnloadedCell((0, 63, 0)))
    ));
}
