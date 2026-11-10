//! Fixed-step driver for the bounded water transition.
//!
//! This runtime owns logical time and the ordered scheduled-tick queue only.
//! It does not infer startup schedules or neighbor callbacks; callers must
//! supply a queue created by a captured native rule.

use std::time::Duration;

use beta_core::water::{
    MAX_WATER_TICKS, WaterBlock, WaterCell, WaterError, WaterEvent, WaterPos, WaterTick,
    WaterTickInput, WaterTickResult, transition_water_ticks,
};

pub const WATER_TICK: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CapturedSeedAction {
    PlaceFlowing(WaterPos),
    PlaceAdjacentSolidAndNotify(WaterPos),
    RemoveSourceSupportAndNotify(WaterPos),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterSeedResult {
    pub cells: std::collections::BTreeMap<WaterPos, WaterCell>,
    pub events: Vec<WaterEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterRuntime {
    world_time: i64,
    accumulator: Duration,
    queue: Vec<WaterTick>,
    last_events: Vec<WaterEvent>,
}

impl Default for WaterRuntime {
    fn default() -> Self {
        Self {
            world_time: 0,
            accumulator: Duration::ZERO,
            queue: Vec::new(),
            last_events: Vec::new(),
        }
    }
}

impl WaterRuntime {
    #[must_use]
    pub fn world_time(&self) -> i64 {
        self.world_time
    }

    #[must_use]
    pub fn queue(&self) -> &[WaterTick] {
        &self.queue
    }

    #[must_use]
    pub fn last_events(&self) -> &[WaterEvent] {
        &self.last_events
    }

    pub fn accumulate(&mut self, elapsed: Duration) {
        self.accumulator = self.accumulator.saturating_add(elapsed);
    }

    #[must_use]
    pub fn ticks_due(&self) -> u128 {
        self.accumulator.as_nanos() / WATER_TICK.as_nanos()
    }

    pub fn clear_clock(&mut self) {
        self.accumulator = Duration::ZERO;
    }

    /// Applies one of the three explicitly captured native queue-seeding
    /// operations. The operation and queue commit atomically. These cases do
    /// not define general world-load or neighbor-notification behavior.
    pub fn apply_captured_seed(
        &mut self,
        cells: std::collections::BTreeMap<WaterPos, WaterCell>,
        action: CapturedSeedAction,
    ) -> Result<WaterSeedResult, WaterError> {
        let mut next_queue = self.queue.clone();
        let mut next_cells = cells;
        let mut events = Vec::new();
        match action {
            CapturedSeedAction::PlaceFlowing(position) => {
                let before = loaded(&next_cells, position)?;
                if before.id != 0 {
                    return Err(WaterError::UnsupportedTransition {
                        position,
                        id: before.id,
                        metadata: before.metadata,
                    });
                }
                schedule_request(position, 8, self.world_time, &mut next_queue, &mut events)?;
                edit_cell(
                    &mut next_cells,
                    position,
                    WaterBlock { id: 8, metadata: 0 },
                    &mut events,
                )?;
            }
            CapturedSeedAction::PlaceAdjacentSolidAndNotify(position) => {
                let before = loaded(&next_cells, position)?;
                let water = (
                    position
                        .0
                        .checked_sub(1)
                        .ok_or(WaterError::CoordinateOverflow)?,
                    position.1,
                    position.2,
                );
                let support = (
                    water.0,
                    water
                        .1
                        .checked_sub(1)
                        .ok_or(WaterError::CoordinateOverflow)?,
                    water.2,
                );
                let source = loaded(&next_cells, water)?;
                if before.id != 0
                    || source.id != 9
                    || source.metadata != 0
                    || loaded(&next_cells, support)?.id != 1
                {
                    return Ok(WaterSeedResult {
                        cells: next_cells,
                        events,
                    });
                }
                edit_cell(
                    &mut next_cells,
                    position,
                    WaterBlock { id: 1, metadata: 0 },
                    &mut events,
                )?;
                events.push(WaterEvent::Neighbor { position, id: 1 });
                schedule_request(water, 8, self.world_time, &mut next_queue, &mut events)?;
                edit_cell(
                    &mut next_cells,
                    water,
                    WaterBlock { id: 8, metadata: 0 },
                    &mut events,
                )?;
                schedule_request(water, 8, self.world_time, &mut next_queue, &mut events)?;
            }
            CapturedSeedAction::RemoveSourceSupportAndNotify(position) => {
                let before = loaded(&next_cells, position)?;
                let water = (
                    position.0,
                    position
                        .1
                        .checked_add(1)
                        .ok_or(WaterError::CoordinateOverflow)?,
                    position.2,
                );
                let source = loaded(&next_cells, water)?;
                if before.id != 1 || source.id != 9 || source.metadata != 0 {
                    return Ok(WaterSeedResult {
                        cells: next_cells,
                        events,
                    });
                }
                edit_cell(
                    &mut next_cells,
                    position,
                    WaterBlock { id: 0, metadata: 0 },
                    &mut events,
                )?;
                events.push(WaterEvent::Neighbor { position, id: 0 });
                schedule_request(water, 8, self.world_time, &mut next_queue, &mut events)?;
                edit_cell(
                    &mut next_cells,
                    water,
                    WaterBlock { id: 8, metadata: 0 },
                    &mut events,
                )?;
                schedule_request(water, 8, self.world_time, &mut next_queue, &mut events)?;
            }
        }
        self.queue = next_queue;
        self.last_events = events.clone();
        Ok(WaterSeedResult {
            cells: next_cells,
            events,
        })
    }

    /// Processes one fixed simulation tick. Clock and queue commit together
    /// with the candidate transition; errors leave all three unchanged.
    pub fn step(
        &mut self,
        cells: std::collections::BTreeMap<(i32, i32, i32), WaterCell>,
    ) -> Result<WaterTickResult, WaterError> {
        let next_time = self
            .world_time
            .checked_add(1)
            .ok_or(WaterError::TimeOverflow)?;
        let result = transition_water_ticks(WaterTickInput {
            world_time: next_time,
            cells,
            queue: self.queue.clone(),
            max_ticks: MAX_WATER_TICKS,
        })?;
        self.world_time = next_time;
        self.queue = result.queue_after.clone();
        self.accumulator = self.accumulator.saturating_sub(WATER_TICK);
        self.last_events = result.events.clone();
        Ok(result)
    }
}

fn loaded(
    cells: &std::collections::BTreeMap<WaterPos, WaterCell>,
    position: WaterPos,
) -> Result<WaterBlock, WaterError> {
    match cells.get(&position) {
        Some(WaterCell::Loaded(block)) => Ok(*block),
        Some(WaterCell::Unloaded) => Err(WaterError::UnloadedCell(position)),
        None => Err(WaterError::UnspecifiedCell(position)),
    }
}

fn edit_cell(
    cells: &mut std::collections::BTreeMap<WaterPos, WaterCell>,
    position: WaterPos,
    after: WaterBlock,
    events: &mut Vec<WaterEvent>,
) -> Result<(), WaterError> {
    let before = loaded(cells, position)?;
    if before != after {
        cells.insert(position, WaterCell::Loaded(after));
        events.push(WaterEvent::Edit {
            position,
            before,
            after,
        });
    }
    Ok(())
}

fn schedule_request(
    position: WaterPos,
    id: u8,
    world_time: i64,
    queue: &mut Vec<WaterTick>,
    events: &mut Vec<WaterEvent>,
) -> Result<(), WaterError> {
    const DELAY: i32 = 5;
    let due_time = world_time
        .checked_add(i64::from(DELAY))
        .ok_or(WaterError::TimeOverflow)?;
    events.push(WaterEvent::Schedule {
        position,
        id,
        delay: DELAY,
    });
    if !queue
        .iter()
        .any(|tick| tick.position == position && tick.id == id && tick.due_time == due_time)
    {
        if queue.len() >= MAX_WATER_TICKS {
            return Err(WaterError::TickLimitExceeded {
                limit: MAX_WATER_TICKS,
            });
        }
        queue.push(WaterTick {
            position,
            id,
            due_time,
        });
        queue.sort_by_key(|tick| tick.due_time);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use beta_core::water::{MAX_WATER_TICKS, WaterBlock, WaterCell, WaterEvent, WaterTick};

    use super::{CapturedSeedAction, WATER_TICK, WaterRuntime};

    #[test]
    fn fixed_steps_are_repeatable_and_pause_does_not_advance_logical_time() {
        let mut split = WaterRuntime::default();
        split.accumulate(WATER_TICK / 2);
        assert_eq!(split.ticks_due(), 0);
        split.accumulate(WATER_TICK / 2);
        assert_eq!(split.ticks_due(), 1);
        let mut all = WaterRuntime::default();
        all.accumulate(WATER_TICK);
        assert_eq!(
            split.step(BTreeMap::new()).unwrap(),
            all.step(BTreeMap::new()).unwrap()
        );
        assert_eq!(split.world_time(), 1);
        assert_eq!(split.ticks_due(), 0);
    }

    #[test]
    fn native_flow_tick_changes_only_the_fluid_mesh_in_its_touched_chunk() {
        use std::collections::BTreeMap;

        use beta_core::{
            chunk::{BlockData, ChunkData},
            water::{WaterCell, WaterTick},
        };
        use beta_view::{MeshLimits, Scene, SceneBounds, SceneChunk};

        fn scene(cells: &BTreeMap<(i32, i32, i32), WaterCell>) -> Scene {
            let mut chunks = [
                ((-1, -1), ChunkData::new()),
                ((-1, 0), ChunkData::new()),
                ((-1, 1), ChunkData::new()),
                ((0, -1), ChunkData::new()),
                ((0, 0), ChunkData::new()),
                ((0, 1), ChunkData::new()),
                ((1, -1), ChunkData::new()),
                ((1, 0), ChunkData::new()),
                ((1, 1), ChunkData::new()),
            ]
            .into_iter()
            .collect::<BTreeMap<_, _>>();
            for (&(x, y, z), cell) in cells {
                let WaterCell::Loaded(block) = cell else {
                    continue;
                };
                if block.id == 0 {
                    continue;
                }
                let (cx, lx) = beta_core::chunk::split_block_coord(x);
                let (cz, lz) = beta_core::chunk::split_block_coord(z);
                chunks
                    .get_mut(&(cx, cz))
                    .expect("fixture cell is in a loaded chunk")
                    .set(
                        lx,
                        y as u8,
                        lz,
                        BlockData {
                            id: block.id,
                            metadata: block.metadata,
                            ..BlockData::default()
                        },
                    )
                    .unwrap();
            }
            Scene::new(
                SceneBounds::new(-1, -1, 3, 3).unwrap(),
                chunks
                    .into_iter()
                    .map(|((x, z), data)| SceneChunk { x, z, data })
                    .collect(),
                vec![],
            )
            .unwrap()
        }

        let mut cells = BTreeMap::new();
        for x in -1..=1 {
            for y in 63..=65 {
                for z in -1..=1 {
                    cells.insert(
                        (x, y, z),
                        WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
                    );
                }
            }
        }
        for x in -1..=1 {
            for z in -1..=1 {
                if (x, z) != (0, 0) {
                    cells.insert(
                        (x, 63, z),
                        WaterCell::Loaded(WaterBlock { id: 1, metadata: 0 }),
                    );
                }
            }
        }
        for position in [(0, 63, 0), (-1, 64, 0), (1, 64, 0), (0, 64, -1), (0, 64, 1)] {
            cells.insert(
                position,
                WaterCell::Loaded(WaterBlock { id: 1, metadata: 0 }),
            );
        }
        cells.insert(
            (0, 64, 0),
            WaterCell::Loaded(WaterBlock { id: 8, metadata: 3 }),
        );
        cells.insert(
            (16, 64, 0),
            WaterCell::Loaded(WaterBlock { id: 9, metadata: 0 }),
        );
        let before = scene(&cells);
        let before_touched = before
            .fluid_mesh_chunks(MeshLimits::default(), &[(0, 0)])
            .unwrap();
        let before_unrelated = before
            .fluid_mesh_chunks(MeshLimits::default(), &[(1, 0)])
            .unwrap();

        let mut runtime = WaterRuntime {
            queue: vec![WaterTick {
                position: (0, 64, 0),
                id: 8,
                due_time: 5,
            }],
            ..WaterRuntime::default()
        };
        let mut result = None;
        for _ in 0..5 {
            let step = runtime.step(cells).unwrap();
            cells = step.cells.clone();
            result = Some(step);
        }
        let result = result.unwrap();
        assert_eq!(
            result.cells[&(0, 64, 0)],
            WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 })
        );
        let after = scene(&result.cells);
        let after_touched = after
            .fluid_mesh_chunks(MeshLimits::default(), &[(0, 0)])
            .unwrap();
        let after_unrelated = after
            .fluid_mesh_chunks(MeshLimits::default(), &[(1, 0)])
            .unwrap();
        assert_ne!(before_touched, after_touched);
        assert_eq!(before_unrelated, after_unrelated);
    }

    #[test]
    fn a_failed_transition_preserves_time_queue_and_accumulator() {
        let mut runtime = WaterRuntime {
            queue: vec![WaterTick {
                position: (0, 64, 0),
                id: 8,
                due_time: 1,
            }],
            ..WaterRuntime::default()
        };
        runtime.accumulate(WATER_TICK);
        let before = runtime.clone();
        let cells = BTreeMap::from([(
            (0, 64, 0),
            WaterCell::Loaded(WaterBlock { id: 8, metadata: 0 }),
        )]);
        assert!(runtime.step(cells).is_err());
        assert_eq!(runtime, before);
    }

    #[test]
    fn every_fixed_step_consumes_the_whole_due_batch_in_order() {
        let mut runtime = WaterRuntime {
            queue: vec![
                WaterTick {
                    position: (0, 64, 0),
                    id: 9,
                    due_time: 1,
                },
                WaterTick {
                    position: (1, 64, 0),
                    id: 9,
                    due_time: 1,
                },
            ],
            ..WaterRuntime::default()
        };
        // Still-water ticks are valid no-op native rows. Preserve the exact
        // same-time queue order while consuming both due entries together.
        let cells = BTreeMap::from([
            (
                (0, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 9, metadata: 0 }),
            ),
            (
                (1, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 9, metadata: 0 }),
            ),
        ]);
        let result = runtime.step(cells).unwrap();
        assert_eq!(result.ticks_processed, 2);
        assert!(result.queue_after.is_empty());
        assert!(runtime.queue().is_empty());
        assert_eq!(runtime.world_time(), 1);
    }

    #[test]
    fn captured_duplicate_requests_remain_visible_but_queue_entry_is_unique() {
        let cells = BTreeMap::from([
            (
                (0, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 9, metadata: 0 }),
            ),
            (
                (0, 63, 0),
                WaterCell::Loaded(WaterBlock { id: 1, metadata: 0 }),
            ),
            (
                (1, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
            ),
        ]);
        let mut runtime = WaterRuntime::default();
        let result = runtime
            .apply_captured_seed(
                cells,
                CapturedSeedAction::PlaceAdjacentSolidAndNotify((1, 64, 0)),
            )
            .unwrap();
        assert_eq!(
            result
                .events
                .iter()
                .filter(|event| matches!(event, WaterEvent::Schedule { .. }))
                .count(),
            2
        );
        assert_eq!(runtime.queue().len(), 1);
        assert_eq!(runtime.queue()[0].due_time, 5);
    }

    #[test]
    fn unsupported_branch_and_seed_queue_limit_are_atomic() {
        let mut runtime = WaterRuntime::default();
        runtime
            .apply_captured_seed(
                BTreeMap::from([(
                    (5, 64, 0),
                    WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
                )]),
                CapturedSeedAction::PlaceFlowing((5, 64, 0)),
            )
            .unwrap();
        let cells = BTreeMap::from([
            (
                (0, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 9, metadata: 0 }),
            ),
            (
                (0, 63, 0),
                WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
            ),
            (
                (1, 64, 0),
                WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
            ),
        ]);
        let unchanged = runtime.clone();
        let result = runtime
            .apply_captured_seed(
                cells,
                CapturedSeedAction::PlaceAdjacentSolidAndNotify((1, 64, 0)),
            )
            .unwrap();
        assert!(result.events.is_empty());
        assert_eq!(runtime, unchanged);

        runtime.queue = (0..MAX_WATER_TICKS)
            .map(|index| WaterTick {
                position: (index as i32 + 10, 64, 0),
                id: 8,
                due_time: index as i64 + 1,
            })
            .collect();
        let before = runtime.clone();
        let cells = BTreeMap::from([(
            (0, 64, 0),
            WaterCell::Loaded(WaterBlock { id: 0, metadata: 0 }),
        )]);
        assert!(
            runtime
                .apply_captured_seed(cells, CapturedSeedAction::PlaceFlowing((0, 64, 0)))
                .is_err()
        );
        assert_eq!(runtime, before);
    }
}
