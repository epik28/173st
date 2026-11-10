//! Reads one bounded water-tick batch from stdin and emits native-style TSV.
//!
//! Input records are `cell x y z id metadata`, `unloaded x y z`,
//! `queue x y z id absolute_due_time`, and one `run world_time max_ticks`.
//! Unloaded cells are rejected because the frozen native corpus has no such row.
//! Output begins with a processed-tick count, preserves ordered transition
//! events, then prints the complete queue and loaded cell state. The probe has
//! no clock or filesystem access.

use std::{
    collections::BTreeMap,
    error::Error,
    io::{self, BufRead},
};

use beta_core::water::{
    WaterBlock, WaterCell, WaterPos, WaterTick, WaterTickInput, transition_water_ticks,
};

fn main() -> Result<(), Box<dyn Error>> {
    let mut cells = BTreeMap::<WaterPos, WaterCell>::new();
    let mut queue = Vec::new();
    let mut run = None;
    for (line_number, line) in io::stdin().lock().lines().enumerate() {
        let line = line?;
        if line.trim().is_empty() || line.starts_with('#') {
            continue;
        }
        let fields: Vec<_> = line.split('\t').collect();
        match fields[0] {
            "cell" if fields.len() == 6 => {
                let position = position(&fields[1..4])?;
                let cell = WaterCell::Loaded(WaterBlock {
                    id: fields[4].parse()?,
                    metadata: fields[5].parse()?,
                });
                if cells.insert(position, cell).is_some() {
                    return Err(format!("duplicate cell at input line {}", line_number + 1).into());
                }
            }
            "unloaded" if fields.len() == 4 => {
                let _position = position(&fields[1..4])?;
                return Err(format!(
                    "unloaded cells are outside the captured probe contract (line {})",
                    line_number + 1
                )
                .into());
            }
            "queue" if fields.len() == 6 => queue.push(WaterTick {
                position: position(&fields[1..4])?,
                id: fields[4].parse()?,
                due_time: fields[5].parse()?,
            }),
            "run" if fields.len() == 3 => {
                if run.is_some() {
                    return Err("probe accepts exactly one run record".into());
                }
                run = Some((fields[1].parse()?, fields[2].parse()?));
            }
            _ => return Err(format!("invalid input record at line {}", line_number + 1).into()),
        }
    }
    let (world_time, max_ticks) = run.ok_or("missing run record")?;
    let result = transition_water_ticks(WaterTickInput {
        world_time,
        cells,
        queue,
        max_ticks,
    })?;
    println!("result\tprocessed_ticks={}", result.ticks_processed);
    for event in result.events {
        match event {
            beta_core::water::WaterEvent::Edit {
                position,
                before,
                after,
            } => println!(
                "edit\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
                position.0,
                position.1,
                position.2,
                before.id,
                before.metadata,
                after.id,
                after.metadata
            ),
            beta_core::water::WaterEvent::Schedule {
                position,
                id,
                delay,
            } => println!(
                "schedule\t{}\t{}\t{}\t{id}\t{delay}",
                position.0, position.1, position.2
            ),
            beta_core::water::WaterEvent::Neighbor { position, id } => println!(
                "neighbor\t{}\t{}\t{}\t{id}",
                position.0, position.1, position.2
            ),
        }
    }
    let queue_after = result
        .queue_after
        .iter()
        .map(|tick| {
            format!(
                "{}:{}:{}:{}:{}",
                tick.position.0, tick.position.1, tick.position.2, tick.id, tick.due_time
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    println!("queue_after\t{queue_after}");
    // Frozen C01 state uses descending y, then z, then x.
    for y in (62..=65).rev() {
        for z in -2..=2 {
            for x in -2..=2 {
                let position = (x, y, z);
                if let Some(cell) = result.cells.get(&position) {
                    print_cell(position, *cell);
                }
            }
        }
    }
    for (position, cell) in &result.cells {
        if !(62..=65).contains(&position.1)
            || !(-2..=2).contains(&position.0)
            || !(-2..=2).contains(&position.2)
        {
            print_cell(*position, *cell);
        }
    }
    Ok(())
}

fn position(fields: &[&str]) -> Result<WaterPos, Box<dyn Error>> {
    Ok((fields[0].parse()?, fields[1].parse()?, fields[2].parse()?))
}

fn print_cell(position: WaterPos, cell: WaterCell) {
    match cell {
        WaterCell::Loaded(block) => println!(
            "state\t{}\t{}\t{}\t{}\t{}",
            position.0, position.1, position.2, block.id, block.metadata
        ),
        WaterCell::Unloaded => println!("unloaded\t{}\t{}\t{}", position.0, position.1, position.2),
    }
}
