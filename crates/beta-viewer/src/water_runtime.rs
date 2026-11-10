//! Fixed-step driver for the bounded water transition.
//!
//! This runtime owns logical time and the ordered scheduled-tick queue only.
//! It does not infer startup schedules or neighbor callbacks; callers must
//! supply a queue created by a captured native rule.

use std::time::Duration;

use beta_core::water::{
    MAX_WATER_TICKS, WaterCell, WaterError, WaterTick, WaterTickInput, WaterTickResult,
    transition_water_ticks,
};

pub const WATER_TICK: Duration = Duration::from_millis(50);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WaterRuntime {
    world_time: i64,
    accumulator: Duration,
    queue: Vec<WaterTick>,
}

impl Default for WaterRuntime {
    fn default() -> Self {
        Self {
            world_time: 0,
            accumulator: Duration::ZERO,
            queue: Vec::new(),
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
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use beta_core::water::{WaterBlock, WaterCell, WaterTick};

    use super::{WATER_TICK, WaterRuntime};

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
}
