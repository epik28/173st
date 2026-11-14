//! Single-player controller state corresponding to native `os.c` and `os.a`.
#[derive(Debug, Default)]
pub struct MiningRuntime {
    target: Option<(i32, i32, i32)>,
    progress: f32,
    cooldown: u8,
}

impl MiningRuntime {
    /// Release or loss of focus resets the native accumulated damage and delay.
    pub fn reset(&mut self) {
        self.progress = 0.0;
        self.cooldown = 0;
    }

    /// Returns true when the targeted block should be destroyed this tick.
    pub fn tick(&mut self, target: (i32, i32, i32), increment: f32) -> bool {
        if self.cooldown > 0 {
            self.cooldown -= 1;
            return false;
        }
        if self.target != Some(target) {
            self.target = Some(target);
            self.progress = 0.0;
            return false;
        }
        self.progress += increment;
        if self.progress >= 1.0 {
            self.progress = 0.0;
            self.cooldown = 5;
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn target_change_resets_progress_and_break_has_five_tick_delay() {
        let mut state = MiningRuntime::default();
        assert!(!state.tick((0, 0, 0), 0.4));
        assert!(!state.tick((0, 0, 0), 0.4));
        assert!(!state.tick((1, 0, 0), 0.4));
        assert!(!state.tick((1, 0, 0), 0.4));
        assert!(!state.tick((1, 0, 0), 0.4));
        assert!(state.tick((1, 0, 0), 0.4));
        for _ in 0..5 {
            assert!(!state.tick((1, 0, 0), 1.0));
        }
        assert!(state.tick((1, 0, 0), 1.0));
    }
    #[test]
    fn release_clears_progress_and_cooldown() {
        let mut state = MiningRuntime::default();
        state.tick((0, 0, 0), 0.6);
        state.tick((0, 0, 0), 0.6);
        state.reset();
        assert!(!state.tick((0, 0, 0), 0.6));
        assert!(state.tick((0, 0, 0), 0.6));
        state.reset();
        assert!(state.tick((0, 0, 0), 1.0));
    }
}
