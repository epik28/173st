//! Focused inventory-screen state and keyboard transition rules.

use super::hud::{Rect, quad_vertices};
use beta_core::items::Inventory;

pub const PANEL_WIDTH: f32 = 176.0;
pub const PANEL_HEIGHT: f32 = 166.0;
pub const MAIN_SLOT_COUNT: usize = 36;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InventoryLayout {
    pub panel: Rect,
    pub slots: [Rect; MAIN_SLOT_COUNT],
}

pub struct InventoryVertices {
    pub panel: Vec<[f32; 4]>,
    pub items: Vec<[f32; 4]>,
}

/// Native centered inventory panel and 36 main slot positions, scaled in
/// integer GUI pixels and emitted in physical viewport pixels.
pub fn native_layout(width: u32, height: u32, scale: u32) -> InventoryLayout {
    let scale = scale.max(1);
    let logical_width = width.div_ceil(scale);
    let logical_height = height.div_ceil(scale);
    let origin_x = (logical_width as i32 / 2 - 88) as f32 * scale as f32;
    let origin_y = (logical_height as i32 / 2 - 83) as f32 * scale as f32;
    let panel = Rect {
        x: origin_x,
        y: origin_y,
        width: PANEL_WIDTH * scale as f32,
        height: PANEL_HEIGHT * scale as f32,
    };
    let slots = std::array::from_fn(|slot| {
        let column = slot % 9;
        let relative_y = if slot < 9 {
            142.0
        } else {
            84.0 + ((slot - 9) / 9 * 18) as f32
        };
        Rect {
            x: origin_x + (8 + column * 18) as f32 * scale as f32,
            y: origin_y + relative_y * scale as f32,
            width: 16.0 * scale as f32,
            height: 16.0 * scale as f32,
        }
    });
    InventoryLayout { panel, slots }
}

/// Return the audited 16x16 item-atlas tile for a supported stack identity.
pub fn item_icon_uv(id: u16, damage: u16) -> Result<Rect, &'static str> {
    if damage != 0 {
        return Err("unsupported inventory item damage");
    }
    let (x, y) = match id {
        1 => (16.0, 0.0),
        3 => (32.0, 0.0),
        332 => (224.0, 0.0),
        267 => (32.0, 64.0),
        _ => return Err("unsupported inventory item ID"),
    };
    Ok(Rect {
        x,
        y,
        width: 16.0,
        height: 16.0,
    })
}

/// Build the background quad and icon quads for the current inventory. Empty
/// and unsupported slots never receive a fabricated replacement icon.
pub fn native_vertices(
    inventory: &Inventory,
    width: u32,
    height: u32,
    scale: u32,
) -> Result<InventoryVertices, &'static str> {
    let layout = native_layout(width, height, scale);
    let panel = quad_vertices(
        layout.panel,
        Rect {
            x: 0.0,
            y: 0.0,
            width: PANEL_WIDTH,
            height: PANEL_HEIGHT,
        },
        (width, height),
    );
    let mut items = Vec::with_capacity(MAIN_SLOT_COUNT * 6);
    for slot in 0..MAIN_SLOT_COUNT {
        if let Some(stack) = inventory
            .stack_at(slot)
            .map_err(|_| "invalid inventory slot")?
        {
            let uv = item_icon_uv(stack.id(), stack.damage())?;
            items.extend(quad_vertices(layout.slots[slot], uv, (width, height)));
        }
    }
    Ok(InventoryVertices {
        panel: panel.to_vec(),
        items,
    })
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct InventoryScreen {
    open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Unchanged,
    Opened,
    Closed,
    ReleasedCapture,
}

impl InventoryScreen {
    pub const fn is_open(self) -> bool {
        self.open
    }

    /// Apply a physical E/Escape key event. Only focused, non-repeated key
    /// presses change the panel state. E opens; Escape closes and otherwise
    /// retains its existing capture release behavior.
    pub fn key_event(
        &mut self,
        is_e: bool,
        is_escape: bool,
        pressed: bool,
        repeat: bool,
        focused: bool,
    ) -> Transition {
        if !focused || !pressed || repeat {
            return Transition::Unchanged;
        }
        if self.open && is_escape {
            self.open = false;
            return Transition::Closed;
        }
        if !self.open && is_e {
            self.open = true;
            return Transition::Opened;
        }
        if !self.open && is_escape {
            return Transition::ReleasedCapture;
        }
        Transition::Unchanged
    }

    pub fn focus_lost(&mut self) -> bool {
        std::mem::replace(&mut self.open, false)
    }
}

#[cfg(test)]
mod tests {
    use super::{InventoryScreen, Transition};
    use super::{Rect, item_icon_uv, native_layout, native_vertices};
    use beta_core::items::{Inventory, ItemStack};

    #[test]
    fn e_opens_and_escape_closes_only_on_focused_nonrepeat_press() {
        let mut screen = InventoryScreen::default();
        for event in [
            (true, false, true, true, true),
            (true, false, false, false, true),
            (true, false, true, false, false),
        ] {
            assert_eq!(
                screen.key_event(event.0, event.1, event.2, event.3, event.4),
                Transition::Unchanged
            );
            assert!(!screen.is_open());
        }
        assert_eq!(
            screen.key_event(true, false, true, false, true),
            Transition::Opened
        );
        assert!(screen.is_open());
        assert_eq!(
            screen.key_event(false, true, true, true, true),
            Transition::Unchanged
        );
        assert_eq!(
            screen.key_event(true, false, true, false, true),
            Transition::Unchanged
        );
        assert!(screen.is_open());
        assert_eq!(
            screen.key_event(false, true, true, false, true),
            Transition::Closed
        );
        assert!(!screen.is_open());
    }

    #[test]
    fn escape_when_closed_requests_existing_mouse_capture_release() {
        let mut screen = InventoryScreen::default();
        assert_eq!(
            screen.key_event(false, true, true, false, true),
            Transition::ReleasedCapture
        );
    }

    #[test]
    fn focus_loss_closes_open_panel_and_is_idempotent() {
        let mut screen = InventoryScreen::default();
        assert_eq!(
            screen.key_event(true, false, true, false, true),
            Transition::Opened
        );
        assert!(screen.focus_lost());
        assert!(!screen.is_open());
        assert!(!screen.focus_lost());
    }

    #[test]
    fn gameplay_input_is_blocked_only_while_the_screen_is_open() {
        let mut screen = InventoryScreen::default();
        assert!(!screen.is_open());
        assert_eq!(
            screen.key_event(true, false, true, false, true),
            Transition::Opened
        );
        assert!(screen.is_open());
        assert_eq!(
            screen.key_event(false, true, true, false, true),
            Transition::Closed
        );
        assert!(!screen.is_open());
    }

    #[test]
    fn inventory_panel_and_slots_scale_from_native_gui_coordinates() {
        for (width, height, scale) in [(1280, 720, 3), (1920, 1080, 4), (640, 480, 2)] {
            let layout = native_layout(width, height, scale);
            assert_eq!(layout.panel.width, 176.0 * scale as f32);
            assert_eq!(layout.panel.height, 166.0 * scale as f32);
            assert_eq!(
                layout.panel.x + layout.panel.width / 2.0,
                (width.div_ceil(scale) / 2) as f32 * scale as f32
            );
            assert_eq!(
                layout.panel.y + layout.panel.height / 2.0,
                (height.div_ceil(scale) / 2) as f32 * scale as f32
            );
            assert_eq!(layout.slots[0].x, layout.panel.x + 8.0 * scale as f32);
            assert_eq!(layout.slots[0].y, layout.panel.y + 142.0 * scale as f32);
            assert_eq!(
                layout.slots[8].x,
                layout.slots[0].x + 8.0 * 18.0 * scale as f32
            );
            assert_eq!(layout.slots[9].x, layout.slots[0].x);
            assert_eq!(layout.slots[9].y, layout.panel.y + 84.0 * scale as f32);
            assert_eq!(layout.slots[35].x, layout.slots[8].x);
            assert_eq!(layout.slots[35].y, layout.panel.y + 120.0 * scale as f32);
            assert_eq!(layout.slots[0].width, 16.0 * scale as f32);
            for column in 0..9 {
                assert_eq!(
                    layout.slots[column].x,
                    layout.panel.x + (8 + column * 18) as f32 * scale as f32
                );
                assert_eq!(
                    layout.slots[column].y,
                    layout.panel.y + 142.0 * scale as f32
                );
                for row in 0..3 {
                    let slot = 9 + row * 9 + column;
                    assert_eq!(
                        layout.slots[slot].x,
                        layout.panel.x + (8 + column * 18) as f32 * scale as f32
                    );
                    assert_eq!(
                        layout.slots[slot].y,
                        layout.panel.y + (84 + row * 18) as f32 * scale as f32
                    );
                }
            }
        }
    }

    #[test]
    fn item_tiles_are_bounded_and_unsupported_identity_or_damage_fails() {
        for (id, expected) in [
            (1, (16.0, 0.0)),
            (3, (32.0, 0.0)),
            (332, (224.0, 0.0)),
            (267, (32.0, 64.0)),
        ] {
            let uv = item_icon_uv(id, 0).unwrap();
            assert_eq!((uv.x, uv.y), expected);
            assert!(uv.x >= 0.0 && uv.y >= 0.0);
            assert!(uv.x + uv.width <= 256.0 && uv.y + uv.height <= 256.0);
        }
        assert!(item_icon_uv(999, 0).is_err());
        assert!(item_icon_uv(1, 1).is_err());
    }

    #[test]
    fn empty_inventory_draws_only_panel_and_supported_stack_draws_one_icon() {
        let mut inventory = Inventory::new();
        let vertices = native_vertices(&inventory, 1280, 720, 3).unwrap();
        assert_eq!(vertices.panel.len(), 6);
        assert!(vertices.items.is_empty());
        assert_eq!(&vertices.panel[0][2..], &[0.0, 0.0]);
        assert_eq!(&vertices.panel[2][2..], &[176.0 / 256.0, 166.0 / 256.0]);
        assert!(vertices.panel.iter().all(|vertex| {
            (0.0..=1.0).contains(&vertex[2]) && (0.0..=1.0).contains(&vertex[3])
        }));

        inventory
            .set_slot(17, Some(ItemStack::new(332, 4, 0).unwrap()))
            .unwrap();
        let vertices = native_vertices(&inventory, 1280, 720, 3).unwrap();
        assert_eq!(vertices.panel.len(), 6);
        assert_eq!(vertices.items.len(), 6);
        assert_eq!(vertices.items[0][2], 224.0 / 256.0);
        assert_eq!(vertices.items[0][3], 0.0);
        assert!(vertices.items.iter().all(|vertex| {
            (0.0..=1.0).contains(&vertex[2]) && (0.0..=1.0).contains(&vertex[3])
        }));
    }

    #[test]
    fn all_four_supported_item_ids_render_their_native_tiles_in_slot_order() {
        let mut inventory = Inventory::new();
        for (slot, id, uv) in [
            (0, 1, (16.0, 0.0)),
            (9, 3, (32.0, 0.0)),
            (20, 332, (224.0, 0.0)),
            (35, 267, (32.0, 64.0)),
        ] {
            inventory
                .set_slot(slot, Some(ItemStack::new(id, 1, 0).unwrap()))
                .unwrap();
            assert_eq!(
                item_icon_uv(id, 0).unwrap(),
                Rect {
                    x: uv.0,
                    y: uv.1,
                    width: 16.0,
                    height: 16.0,
                }
            );
        }
        let vertices = native_vertices(&inventory, 1280, 720, 3).unwrap();
        assert_eq!(vertices.items.len(), 4 * 6);
        for (quad, expected) in [
            (0, (16.0, 0.0)),
            (1, (32.0, 0.0)),
            (2, (224.0, 0.0)),
            (3, (32.0, 64.0)),
        ] {
            assert_eq!(vertices.items[quad * 6][2], expected.0 / 256.0);
            assert_eq!(vertices.items[quad * 6][3], expected.1 / 256.0);
        }
        assert!(item_icon_uv(1, 1).is_err());
        assert!(native_vertices(&inventory, 1280, 720, 3).is_ok());
    }
}
