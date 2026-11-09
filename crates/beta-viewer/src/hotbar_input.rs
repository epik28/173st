//! Pure hotbar input state transitions, separate from window-system events.

use beta_core::items::Inventory;

/// Accumulates sub-notch window-system deltas. Winit line events use one line
/// per notch; pixel events use the fixed 120-pixel wheel unit exposed by
/// common desktop backends. A completed event becomes one native wheel event:
/// the original client clamps the event magnitude to its sign.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct WheelAccumulator {
    line_residual: f64,
    pixel_residual: f64,
}

impl WheelAccumulator {
    pub fn line_event(&mut self, delta: f64) -> Option<i8> {
        Self::accumulate(&mut self.line_residual, delta, 1.0)
    }

    pub fn pixel_event(&mut self, delta: f64) -> Option<i8> {
        Self::accumulate(&mut self.pixel_residual, delta, 120.0)
    }

    pub fn clear(&mut self) {
        self.line_residual = 0.0;
        self.pixel_residual = 0.0;
    }

    fn accumulate(residual: &mut f64, delta: f64, unit: f64) -> Option<i8> {
        if !delta.is_finite() {
            *residual = 0.0;
            return None;
        }
        let total = *residual + delta;
        if total.abs() < unit {
            *residual = total;
            return None;
        }
        // Keep only a same-sign fractional tail. Extra whole units from a
        // single OS event are discarded, matching the native sign clamp.
        *residual = total % unit;
        Some(if total.is_sign_positive() { -1 } else { 1 })
    }
}

/// Apply one completed native wheel event; positive native deltas select the
/// previous slot and negative deltas the next slot, wrapping around 0..8.
pub fn wheel_step(inventory: &mut Inventory, step: i8) {
    let selected = i16::from(inventory.selected_hotbar());
    let next = (selected + i16::from(step)).rem_euclid(9) as i32;
    inventory
        .select_hotbar(next)
        .expect("wrapped hotbar index is in range");
}

/// Apply a physical number-key press to the active inventory.
///
/// `number` is the number printed on the physical key (1 through 9), rather
/// than a text/character event, so keyboard layouts cannot change the mapping.
pub fn number_key(
    inventory: &mut Inventory,
    number: u8,
    pressed: bool,
    repeat: bool,
    focused: bool,
) -> bool {
    if !focused || !pressed || repeat || !(1..=9).contains(&number) {
        return false;
    }
    inventory.select_hotbar(i32::from(number - 1)).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn physical_number_keys_select_all_nine_slots() {
        let mut inventory = Inventory::new();
        for number in 1..=9 {
            assert!(number_key(&mut inventory, number, true, false, true));
            assert_eq!(inventory.selected_hotbar(), number - 1);
        }
    }

    #[test]
    fn key_release_repeat_unfocused_and_invalid_keys_do_not_change_selection() {
        let mut inventory = Inventory::new();
        assert!(number_key(&mut inventory, 7, true, false, true));
        for args in [
            (3, false, false, true),
            (3, true, true, true),
            (3, true, false, false),
            (0, true, false, true),
            (10, true, false, true),
        ] {
            assert!(!number_key(&mut inventory, args.0, args.1, args.2, args.3));
            assert_eq!(inventory.selected_hotbar(), 6);
        }
    }

    #[test]
    fn line_wheel_accumulates_fractional_events_and_wraps_both_directions() {
        let mut inventory = Inventory::new();
        let mut wheel = WheelAccumulator::default();
        assert_eq!(wheel.line_event(0.4), None);
        assert_eq!(wheel.line_event(0.4), None);
        assert_eq!(wheel.line_event(0.2), Some(-1));
        wheel_step(&mut inventory, -1);
        assert_eq!(inventory.selected_hotbar(), 8);
        assert_eq!(wheel.line_event(-1.0), Some(1));
        wheel_step(&mut inventory, 1);
        assert_eq!(inventory.selected_hotbar(), 0);
    }

    #[test]
    fn each_nonzero_wheel_event_is_one_step_regardless_of_magnitude() {
        let mut inventory = Inventory::new();
        let mut wheel = WheelAccumulator::default();
        for _ in 0..3 {
            let step = wheel.line_event(-2.0).expect("nonzero event");
            wheel_step(&mut inventory, step);
        }
        assert_eq!(inventory.selected_hotbar(), 3);
        assert_eq!(wheel.line_event(0.0), None);
    }

    #[test]
    fn pixel_wheel_uses_fixed_native_notch_threshold_and_focus_reset() {
        let mut inventory = Inventory::new();
        let mut wheel = WheelAccumulator::default();
        assert_eq!(wheel.pixel_event(-60.0), None);
        assert_eq!(wheel.pixel_event(-59.5), None);
        let step = wheel.pixel_event(-0.5).expect("one pixel notch");
        wheel_step(&mut inventory, step);
        assert_eq!(inventory.selected_hotbar(), 1);
        assert_eq!(wheel.pixel_event(80.0), None);
        wheel.clear();
        assert_eq!(wheel.pixel_event(40.0), None);
        assert_eq!(inventory.selected_hotbar(), 1);
    }
}
