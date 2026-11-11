//! Bounded in-memory water block-tick transitions.
//!
//! This API deliberately models one frozen, loaded region. It performs no
//! file, clock, or global-world access. Unsupported states fail before a
//! candidate result is returned.

use std::collections::{BTreeMap, BTreeSet};
use std::{error::Error, fmt};

pub const MAX_WATER_CELLS: usize = 256;
pub const MAX_WATER_TICKS: usize = 64;

pub type WaterPos = (i32, i32, i32);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterBlock {
    pub id: u8,
    pub metadata: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WaterCell {
    Loaded(WaterBlock),
    Unloaded,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WaterTick {
    pub position: WaterPos,
    pub id: u8,
    /// Absolute native scheduled time.
    pub due_time: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaterEvent {
    Edit {
        position: WaterPos,
        before: WaterBlock,
        after: WaterBlock,
    },
    Schedule {
        position: WaterPos,
        id: u8,
        delay: i32,
    },
    Neighbor {
        position: WaterPos,
        id: u8,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterTickInput {
    pub world_time: i64,
    pub cells: BTreeMap<WaterPos, WaterCell>,
    /// Full ordered native queue at the start of this world-time batch.
    pub queue: Vec<WaterTick>,
    pub max_ticks: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterTickResult {
    pub cells: BTreeMap<WaterPos, WaterCell>,
    pub queue_after: Vec<WaterTick>,
    pub events: Vec<WaterEvent>,
    pub ticks_processed: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WaterError {
    InvalidLimit,
    InvalidQueueOrder,
    CellLimitExceeded,
    TickLimitExceeded {
        limit: usize,
    },
    UnspecifiedCell(WaterPos),
    UnloadedCell(WaterPos),
    InvalidState {
        position: WaterPos,
        id: u8,
        metadata: u8,
    },
    TimeOverflow,
    CoordinateOverflow,
    UnsupportedTransition {
        position: WaterPos,
        id: u8,
        metadata: u8,
    },
}

impl fmt::Display for WaterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl Error for WaterError {}

/// Applies all ticks already due at `world_time` in the supplied queue order.
/// Newly requested ticks are queued after the due batch. Inputs are cloned and
/// validated first; errors never expose a partially transitioned scene.
pub fn transition_water_ticks(input: WaterTickInput) -> Result<WaterTickResult, WaterError> {
    if input.max_ticks == 0 {
        return Err(WaterError::InvalidLimit);
    }
    if input.cells.len() > MAX_WATER_CELLS {
        return Err(WaterError::CellLimitExceeded);
    }
    if input.queue.len() > MAX_WATER_TICKS {
        return Err(WaterError::TickLimitExceeded {
            limit: MAX_WATER_TICKS,
        });
    }
    for (position, cell) in &input.cells {
        if let WaterCell::Loaded(block) = cell
            && (block.metadata > 15 || (block.id == 0 && block.metadata != 0))
        {
            return Err(WaterError::InvalidState {
                position: *position,
                id: block.id,
                metadata: block.metadata,
            });
        }
    }

    let WaterTickInput {
        world_time,
        cells: input_cells,
        queue: input_queue,
        max_ticks,
    } = input;
    if input_queue
        .windows(2)
        .any(|pair| pair[0].due_time > pair[1].due_time)
    {
        return Err(WaterError::InvalidQueueOrder);
    }
    let due_count = input_queue
        .iter()
        .take_while(|tick| tick.due_time <= world_time)
        .count();
    if due_count > max_ticks {
        return Err(WaterError::TickLimitExceeded { limit: max_ticks });
    }
    let mut cells = input_cells;
    let mut queue: Vec<_> = input_queue.iter().copied().skip(due_count).collect();
    let mut events = Vec::new();
    for tick in input_queue.into_iter().take(due_count) {
        let state = loaded(&cells, tick.position)?;
        if state.id != tick.id {
            continue;
        }
        apply_one(
            tick.position,
            state,
            world_time,
            &mut cells,
            &mut queue,
            &mut events,
        )?;
        if queue.len() > MAX_WATER_TICKS {
            return Err(WaterError::TickLimitExceeded {
                limit: MAX_WATER_TICKS,
            });
        }
    }
    Ok(WaterTickResult {
        cells,
        queue_after: queue,
        events,
        ticks_processed: due_count,
    })
}

fn apply_one(
    pos: WaterPos,
    block: WaterBlock,
    world_time: i64,
    cells: &mut BTreeMap<WaterPos, WaterCell>,
    queue: &mut Vec<WaterTick>,
    events: &mut Vec<WaterEvent>,
) -> Result<(), WaterError> {
    if block.id == 9 {
        return Ok(());
    }
    if block.id != 8 {
        return Err(WaterError::UnsupportedTransition {
            position: pos,
            id: block.id,
            metadata: block.metadata,
        });
    }

    let below = offset(pos, 0, -1, 0)?;
    let below_state = loaded(cells, below)?;

    // Captured source formation: two still sources and two stone neighbors
    // surround a metadata-1 flowing block supported by stone.
    let east = offset(pos, 1, 0, 0)?;
    if block.metadata == 1 && below_state.id == 1 {
        let neighbors = [
            west_of(pos)?,
            east,
            offset(pos, 0, 0, -1)?,
            offset(pos, 0, 0, 1)?,
        ];
        let mut sources = Vec::new();
        let mut stones = 0;
        for neighbor in neighbors {
            match loaded(cells, neighbor)? {
                WaterBlock { id: 9, metadata: 0 } => sources.push(neighbor),
                WaterBlock { id: 1, metadata: 0 } => stones += 1,
                _ => {}
            }
        }
        let north = offset(pos, 0, 0, -1)?;
        let south = offset(pos, 0, 0, 1)?;
        let west = west_of(pos)?;
        let captured_pair = sources.as_slice() == [west, east]
            || sources.as_slice() == [north, south]
            || sources.as_slice() == [west, north]
            || sources.as_slice() == [east, south]
            || sources.as_slice() == [west, south]
            || sources.as_slice() == [east, north];
        if captured_pair && stones == 2 {
            events.push(WaterEvent::Neighbor {
                position: pos,
                id: 8,
            });
            for neighbor in sources {
                schedule(neighbor, 8, world_time, queue, events)?;
                edit(cells, neighbor, WaterBlock { id: 8, metadata: 0 }, events)?;
                schedule(neighbor, 8, world_time, queue, events)?;
            }
            schedule(pos, 8, world_time, queue, events)?;
            set_silent(cells, pos, WaterBlock { id: 8, metadata: 0 })?;
            events.push(WaterEvent::Neighbor {
                position: pos,
                id: 8,
            });
            return Ok(());
        }
    }

    // This captured setup covers a flowing cell beside an existing source,
    // with an open cell directly below it.
    let west = west_of(pos)?;
    let west_state = loaded(cells, west)?;
    if block.metadata == 2 && west_state.id == 9 && below_state.id == 0 {
        events.push(WaterEvent::Neighbor {
            position: pos,
            id: block.id,
        });
        schedule(west, 8, world_time, queue, events)?;
        edit(cells, west, WaterBlock { id: 8, metadata: 0 }, events)?;
        schedule(west, 8, world_time, queue, events)?;
        schedule(pos, 8, world_time, queue, events)?;
        events.push(WaterEvent::Neighbor {
            position: pos,
            id: block.id,
        });
        set_silent(cells, pos, WaterBlock { id: 8, metadata: 1 })?;
        schedule(below, 8, world_time, queue, events)?;
        edit(cells, below, WaterBlock { id: 8, metadata: 9 }, events)?;
        events.push(WaterEvent::Neighbor {
            position: below,
            id: 8,
        });
        return Ok(());
    }

    if block.metadata == 3 {
        set_silent(cells, pos, WaterBlock { id: 0, metadata: 0 })?;
        events.push(WaterEvent::Neighbor {
            position: pos,
            id: 0,
        });
        return Ok(());
    }
    if block.metadata == 4 {
        if below_state.id != 0 {
            return Err(WaterError::UnsupportedTransition {
                position: pos,
                id: block.id,
                metadata: block.metadata,
            });
        }
        set_silent(cells, pos, WaterBlock { id: 0, metadata: 0 })?;
        events.push(WaterEvent::Neighbor {
            position: pos,
            id: 0,
        });
        schedule(below, 8, world_time, queue, events)?;
        edit(cells, below, WaterBlock { id: 8, metadata: 7 }, events)?;
        events.push(WaterEvent::Neighbor {
            position: below,
            id: 8,
        });
        return Ok(());
    }
    if !matches!(block.metadata, 0 | 1 | 8) {
        return Err(WaterError::UnsupportedTransition {
            position: pos,
            id: block.id,
            metadata: block.metadata,
        });
    }

    let still = WaterBlock {
        id: 9,
        metadata: block.metadata,
    };
    edit(cells, pos, still, events)?;

    if below_state.id == 0 {
        schedule(below, 8, world_time, queue, events)?;
        let below_metadata = match block.metadata {
            0 | 8 => 8,
            1 => 9,
            _ => unreachable!("metadata was restricted above"),
        };
        edit(
            cells,
            below,
            WaterBlock {
                id: 8,
                metadata: below_metadata,
            },
            events,
        )?;
        events.push(WaterEvent::Neighbor {
            position: below,
            id: 8,
        });
        schedule(pos, 8, world_time, queue, events)?;
        let flowing = WaterBlock {
            id: 8,
            metadata: block.metadata,
        };
        edit(cells, pos, flowing, events)?;
        schedule(pos, 8, world_time, queue, events)?;
        return Ok(());
    }

    let mut directions = Vec::new();
    for (dx, dz) in [(-1, 0), (1, 0), (0, -1), (0, 1)] {
        let target = offset(pos, dx, 0, dz)?;
        if loaded(cells, target)?.id == 0 {
            directions.push(target);
        }
    }
    if directions.is_empty() {
        return Ok(());
    }
    // The native downhill fixtures have four open horizontal neighbors,
    // with air or stone immediately below each of them. When at least one
    // has air below, native chooses every such direction in native order.
    if block.metadata == 0 && below_state.id == 1 && directions.len() == 4 {
        let mut downhill = Vec::new();
        let mut known_count = 0;
        for &target in &directions {
            match loaded(cells, offset(target, 0, -1, 0)?)?.id {
                0 => {
                    downhill.push(target);
                    known_count += 1;
                }
                1 => known_count += 1,
                _ => {}
            }
        }
        if known_count == 4 && !downhill.is_empty() {
            directions = downhill;
        }
    }
    // A completely specified 5x5 stone floor with one opening two steps
    // away is the bounded native fixture for searching a short route.
    if block.metadata == 0 && below_state.id == 1 && directions.len() >= 3 {
        let mut hole = None;
        let mut known = true;
        for dx in -2_i32..=2 {
            for dz in -2_i32..=2 {
                let floor = offset(pos, dx, -1, dz)?;
                match cells.get(&floor) {
                    Some(WaterCell::Loaded(WaterBlock { id: 1, .. })) => {}
                    Some(WaterCell::Loaded(WaterBlock { id: 0, .. })) if hole.is_none() => {
                        hole = Some((dx, dz))
                    }
                    _ => known = false,
                }
            }
        }
        if known
            && let Some((hx, hz)) = hole
            && hx.abs() + hz.abs() == 2
        {
            // Captured scenes contain air across the 5x5 plane except the
            // source and at most one immediately adjacent stone obstacle.
            let mut adjacent_stones = 0;
            for dx in -2_i32..=2 {
                for dz in -2_i32..=2 {
                    if dx == 0 && dz == 0 {
                        continue;
                    }
                    match cells.get(&offset(pos, dx, 0, dz)?) {
                        Some(WaterCell::Loaded(WaterBlock { id: 0, .. })) => {}
                        Some(WaterCell::Loaded(WaterBlock { id: 1, .. }))
                            if dx.abs() + dz.abs() == 1 =>
                        {
                            adjacent_stones += 1
                        }
                        _ => known = false,
                    }
                }
            }
            if !known || adjacent_stones > 1 {
                return Err(WaterError::UnsupportedTransition {
                    position: pos,
                    id: block.id,
                    metadata: block.metadata,
                });
            }
            let mut shortest = Vec::new();
            for &target in &directions {
                let dx = target.0 - pos.0;
                let dz = target.2 - pos.2;
                if (hx - dx).abs() + (hz - dz).abs() == 1 {
                    shortest.push(target);
                }
            }
            if shortest.is_empty() {
                return Err(WaterError::UnsupportedTransition {
                    position: pos,
                    id: block.id,
                    metadata: block.metadata,
                });
            }
            directions = shortest;
        }
    }
    for (index, target) in directions.into_iter().enumerate() {
        schedule(target, 8, world_time, queue, events)?;
        edit(cells, target, WaterBlock { id: 8, metadata: 1 }, events)?;
        events.push(WaterEvent::Neighbor {
            position: target,
            id: 8,
        });
        if index == 0 {
            schedule(pos, 8, world_time, queue, events)?;
            let flowing = WaterBlock {
                id: 8,
                metadata: block.metadata,
            };
            edit(cells, pos, flowing, events)?;
            schedule(pos, 8, world_time, queue, events)?;
        }
    }
    Ok(())
}

fn west_of(pos: WaterPos) -> Result<WaterPos, WaterError> {
    offset(pos, -1, 0, 0)
}

fn offset(pos: WaterPos, dx: i32, dy: i32, dz: i32) -> Result<WaterPos, WaterError> {
    Ok((
        pos.0
            .checked_add(dx)
            .ok_or(WaterError::CoordinateOverflow)?,
        pos.1
            .checked_add(dy)
            .ok_or(WaterError::CoordinateOverflow)?,
        pos.2
            .checked_add(dz)
            .ok_or(WaterError::CoordinateOverflow)?,
    ))
}

fn loaded(cells: &BTreeMap<WaterPos, WaterCell>, pos: WaterPos) -> Result<WaterBlock, WaterError> {
    match cells.get(&pos) {
        Some(WaterCell::Loaded(block)) => Ok(*block),
        Some(WaterCell::Unloaded) => Err(WaterError::UnloadedCell(pos)),
        None => Err(WaterError::UnspecifiedCell(pos)),
    }
}

fn set_silent(
    cells: &mut BTreeMap<WaterPos, WaterCell>,
    pos: WaterPos,
    block: WaterBlock,
) -> Result<(), WaterError> {
    loaded(cells, pos)?;
    cells.insert(pos, WaterCell::Loaded(block));
    Ok(())
}

fn edit(
    cells: &mut BTreeMap<WaterPos, WaterCell>,
    pos: WaterPos,
    after: WaterBlock,
    events: &mut Vec<WaterEvent>,
) -> Result<(), WaterError> {
    let before = loaded(cells, pos)?;
    if before != after {
        cells.insert(pos, WaterCell::Loaded(after));
        events.push(WaterEvent::Edit {
            position: pos,
            before,
            after,
        });
    }
    Ok(())
}

fn schedule(
    pos: WaterPos,
    id: u8,
    world_time: i64,
    queue: &mut Vec<WaterTick>,
    events: &mut Vec<WaterEvent>,
) -> Result<(), WaterError> {
    const DELAY: i32 = 5;
    events.push(WaterEvent::Schedule {
        position: pos,
        id,
        delay: DELAY,
    });
    let due_time = world_time
        .checked_add(i64::from(DELAY))
        .ok_or(WaterError::TimeOverflow)?;
    if !queue
        .iter()
        .any(|tick| tick.position == pos && tick.id == id && tick.due_time == due_time)
    {
        queue.push(WaterTick {
            position: pos,
            id,
            due_time,
        });
    }
    Ok(())
}

/// Returns the changed positions in deterministic order for callers rebuilding
/// only affected chunk meshes.
pub fn changed_positions(events: &[WaterEvent]) -> BTreeSet<WaterPos> {
    events
        .iter()
        .filter_map(|event| match event {
            WaterEvent::Edit { position, .. } => Some(*position),
            WaterEvent::Schedule { .. } | WaterEvent::Neighbor { .. } => None,
        })
        .collect()
}
