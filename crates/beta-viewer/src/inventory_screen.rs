//! Focused inventory-screen state and keyboard transition rules.

use super::hud::{Rect, quad_vertices};
use beta_core::items::{Inventory, ItemStack};

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
    pub terrain_items: Vec<[f32; 4]>,
    pub items: Vec<[f32; 4]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemAtlas {
    Terrain,
    Items,
}

pub struct CountVertices {
    pub shadow: Vec<[f32; 4]>,
    pub foreground: Vec<[f32; 4]>,
}

const DIGIT_WIDTHS: [u32; 10] = [8, 8, 8, 8, 9, 9, 8, 9, 8, 8];

fn count_width(count: u8) -> u32 {
    count
        .to_string()
        .bytes()
        .map(|digit| DIGIT_WIDTHS[(digit - b'0') as usize])
        .sum()
}

fn count_quads(count: u8, x: f32, y: f32, scale: f32, viewport: (u32, u32)) -> CountVertices {
    let text = count.to_string();
    let width = count_width(count) as f32;
    let start_x = x + (17.0 - width) * scale;
    let mut shadow = Vec::new();
    let mut foreground = Vec::new();
    let mut cursor = start_x;
    for byte in text.bytes() {
        let digit = (byte - b'0') as f32;
        // `quad_vertices` normalizes against the 256-unit HUD convention;
        // double the native 128x128 font pixel coordinates here.
        let uv = Rect {
            x: digit * 16.0,
            y: 48.0,
            width: 16.0,
            height: 16.0,
        };
        let rect = Rect {
            x: cursor,
            y: y + 9.0 * scale,
            width: 8.0 * scale,
            height: 8.0 * scale,
        };
        shadow.extend(quad_vertices(
            Rect {
                x: rect.x + scale,
                y: rect.y + scale,
                ..rect
            },
            uv,
            viewport,
        ));
        foreground.extend(quad_vertices(rect, uv, viewport));
        cursor += DIGIT_WIDTHS[(byte - b'0') as usize] as f32 * scale;
    }
    CountVertices { shadow, foreground }
}

pub fn count_vertices(
    inventory: &Inventory,
    width: u32,
    height: u32,
    scale: u32,
    hotbar: bool,
) -> CountVertices {
    let s = scale.max(1) as f32;
    let mut result = CountVertices {
        shadow: Vec::new(),
        foreground: Vec::new(),
    };
    let layout = super::hud::layout_with_selection(width, height, scale, 0);
    let slots = if hotbar { 9 } else { MAIN_SLOT_COUNT };
    for slot in 0..slots {
        let Ok(Some(stack)) = inventory.stack_at(slot) else {
            continue;
        };
        if stack.count() <= 1 || item_icon_uv(stack.id(), stack.damage()).is_err() {
            continue;
        }
        let (x, y) = if hotbar {
            (
                layout.hotbar.x + (3 + slot * 20) as f32 * s,
                layout.hotbar.y + 3.0 * s,
            )
        } else {
            let slot_rect = native_layout(width, height, scale).slots[slot];
            (slot_rect.x, slot_rect.y)
        };
        let glyphs = count_quads(stack.count(), x, y, s, (width, height));
        result.shadow.extend(glyphs.shadow);
        result.foreground.extend(glyphs.foreground);
    }
    result
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

pub fn item_atlas(id: u16) -> Result<ItemAtlas, &'static str> {
    match id {
        1 | 3 => Ok(ItemAtlas::Terrain),
        332 | 267 => Ok(ItemAtlas::Items),
        _ => Err("unsupported inventory item ID"),
    }
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
    let mut terrain_items = Vec::with_capacity(MAIN_SLOT_COUNT * 6);
    let mut items = Vec::with_capacity(MAIN_SLOT_COUNT * 6);
    for slot in 0..MAIN_SLOT_COUNT {
        if let Some(stack) = inventory
            .stack_at(slot)
            .map_err(|_| "invalid inventory slot")?
        {
            let Ok(uv) = item_icon_uv(stack.id(), stack.damage()) else {
                continue;
            };
            match item_atlas(stack.id()) {
                Ok(ItemAtlas::Terrain) => {
                    terrain_items.extend(quad_vertices(layout.slots[slot], uv, (width, height)))
                }
                Ok(ItemAtlas::Items) => {
                    items.extend(quad_vertices(layout.slots[slot], uv, (width, height)))
                }
                Err(_) => continue,
            }
        }
    }
    Ok(InventoryVertices {
        panel: panel.to_vec(),
        terrain_items,
        items,
    })
}

/// Build the first-person hotbar icon passes. The frame and selection are
/// emitted by `hud`; this function only emits validated saved stacks.
pub fn hotbar_vertices(
    inventory: &Inventory,
    width: u32,
    height: u32,
    scale: u32,
) -> (Vec<[f32; 4]>, Vec<[f32; 4]>) {
    let layout = super::hud::layout_with_selection(width, height, scale, 0);
    let mut terrain = Vec::with_capacity(9 * 6);
    let mut items = Vec::with_capacity(9 * 6);
    for slot in 0..9 {
        let Ok(Some(stack)) = inventory.stack_at(slot) else {
            continue;
        };
        let Ok(uv) = item_icon_uv(stack.id(), stack.damage()) else {
            continue;
        };
        let rect = Rect {
            x: layout.hotbar.x + (3 + slot * 20) as f32 * layout.scale as f32,
            y: layout.hotbar.y + 3.0 * layout.scale as f32,
            width: 16.0 * layout.scale as f32,
            height: 16.0 * layout.scale as f32,
        };
        match item_atlas(stack.id()) {
            Ok(ItemAtlas::Terrain) => terrain.extend(quad_vertices(rect, uv, (width, height))),
            Ok(ItemAtlas::Items) => items.extend(quad_vertices(rect, uv, (width, height))),
            Err(_) => {}
        }
    }
    (terrain, items)
}

pub fn cursor_vertices(
    cursor: Option<ItemStack>,
    width: u32,
    height: u32,
    scale: u32,
    mouse: (f32, f32),
    inventory_origin: (f32, f32),
) -> (Vec<[f32; 4]>, Vec<[f32; 4]>, CountVertices) {
    let Some(stack) = cursor else {
        return (
            Vec::new(),
            Vec::new(),
            CountVertices {
                shadow: Vec::new(),
                foreground: Vec::new(),
            },
        );
    };
    let s = scale.max(1) as f32;
    let rect = Rect {
        x: mouse.0 - inventory_origin.0 - 8.0 * s,
        y: mouse.1 - inventory_origin.1 - 8.0 * s,
        width: 16.0 * s,
        height: 16.0 * s,
    };
    let Ok(uv) = item_icon_uv(stack.id(), stack.damage()) else {
        return (
            Vec::new(),
            Vec::new(),
            CountVertices {
                shadow: Vec::new(),
                foreground: Vec::new(),
            },
        );
    };
    let quad = quad_vertices(rect, uv, (width, height)).to_vec();
    let mut terrain = Vec::new();
    let mut items = Vec::new();
    match item_atlas(stack.id()) {
        Ok(ItemAtlas::Terrain) => terrain.extend(quad),
        Ok(ItemAtlas::Items) => items.extend(quad),
        Err(_) => {}
    }
    let counts = if stack.count() > 1 {
        count_quads(stack.count(), rect.x, rect.y, s, (width, height))
    } else {
        CountVertices {
            shadow: Vec::new(),
            foreground: Vec::new(),
        }
    };
    (terrain, items, counts)
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InventoryScreen {
    open: bool,
    cursor: Option<beta_core::items::ItemStack>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transition {
    Unchanged,
    Opened,
    Closed,
    ReleasedCapture,
}

impl InventoryScreen {
    pub const fn is_open(&self) -> bool {
        self.open
    }

    pub(crate) const fn cursor_stack(&self) -> Option<ItemStack> {
        self.cursor
    }

    pub fn slot_at(&self, width: u32, height: u32, scale: u32, x: f32, y: f32) -> Option<usize> {
        if !self.open {
            return None;
        }
        native_layout(width, height, scale)
            .slots
            .iter()
            .position(|slot| {
                x >= slot.x - scale.max(1) as f32
                    && x < slot.x + slot.width + scale.max(1) as f32
                    && y >= slot.y - scale.max(1) as f32
                    && y < slot.y + slot.height + scale.max(1) as f32
            })
    }

    pub fn click(&mut self, inventory: &mut Inventory, slot: usize, right: bool) -> bool {
        if !self.open || slot >= MAIN_SLOT_COUNT {
            return false;
        }
        let Ok(target) = inventory.stack_at(slot) else {
            return false;
        };
        if right {
            if self.cursor.is_none() {
                let Some(stack) = target else {
                    return false;
                };
                let amount = stack.count().div_ceil(2);
                self.cursor = inventory.split_slot(slot, amount).ok();
                return self.cursor.is_some();
            }
            let mut cursor = self.cursor.take().unwrap();
            match target {
                None => {
                    let placed = cursor.split(1).ok();
                    let _ = inventory.set_slot(slot, placed);
                }
                Some(target)
                    if target.id() == cursor.id()
                        && target.damage() == cursor.damage()
                        && target.count() < target.max_stack() =>
                {
                    let _ = cursor.split(1);
                    let _ = inventory.set_slot(
                        slot,
                        beta_core::items::ItemStack::new(
                            target.id(),
                            target.count() + 1,
                            target.damage(),
                        )
                        .ok(),
                    );
                }
                Some(_) => {}
            }
            if cursor.count() > 0 {
                self.cursor = Some(cursor);
            }
            return true;
        }
        let cursor = self.cursor.take();
        match (cursor, target) {
            (None, Some(_)) => self.cursor = inventory.set_slot(slot, None).ok().flatten(),
            (Some(stack), None) => {
                let _ = inventory.set_slot(slot, Some(stack));
            }
            (Some(mut held), Some(target))
                if held.id() == target.id() && held.damage() == target.damage() =>
            {
                let free = target.max_stack().saturating_sub(target.count());
                let moved = free.min(held.count());
                if moved > 0 {
                    let _ = held.split(moved);
                    let _ = inventory.set_slot(
                        slot,
                        beta_core::items::ItemStack::new(
                            target.id(),
                            target.count() + moved,
                            target.damage(),
                        )
                        .ok(),
                    );
                }
                if held.count() > 0 {
                    self.cursor = Some(held);
                }
            }
            (Some(held), Some(target)) if held.count() <= target.max_stack() => {
                let _ = inventory.set_slot(slot, Some(held));
                self.cursor = Some(target);
            }
            (held, _) => self.cursor = held,
        }
        true
    }

    pub fn return_cursor(&mut self, inventory: &mut Inventory) -> bool {
        if let Some(cursor) = self.cursor.take() {
            let mut incoming = Some(cursor);
            if !matches!(inventory.insert(&mut incoming), Ok(true)) {
                self.cursor = incoming;
                self.open = true;
                return false;
            }
        }
        true
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

    pub fn focus_lost(&mut self, inventory: &mut Inventory) -> bool {
        let was_open = std::mem::replace(&mut self.open, false);
        self.return_cursor(inventory);
        was_open
    }
}

#[cfg(test)]
mod tests {
    use super::{InventoryScreen, Transition};
    use super::{
        MAIN_SLOT_COUNT, Rect, count_vertices, cursor_vertices, hotbar_vertices, item_icon_uv,
        native_layout, native_vertices,
    };
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
        assert!(screen.focus_lost(&mut Inventory::new()));
        assert!(!screen.is_open());
        assert!(!screen.focus_lost(&mut Inventory::new()));
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
        assert_eq!(vertices.terrain_items.len(), 2 * 6);
        assert_eq!(vertices.items.len(), 2 * 6);
        for (batch, expected) in [
            (&vertices.terrain_items, [(16.0, 0.0), (32.0, 0.0)]),
            (&vertices.items, [(224.0, 0.0), (32.0, 64.0)]),
        ] {
            for (quad, expected) in expected.into_iter().enumerate() {
                assert_eq!(batch[quad * 6][2], expected.0 / 256.0);
                assert_eq!(batch[quad * 6][3], expected.1 / 256.0);
            }
        }
        assert!(item_icon_uv(1, 1).is_err());
        assert!(native_vertices(&inventory, 1280, 720, 3).is_ok());
    }

    #[test]
    fn hotbar_icons_use_native_slot_positions_and_atlas_batches() {
        let mut inventory = Inventory::new();
        inventory
            .set_slot(0, Some(ItemStack::new(1, 1, 0).unwrap()))
            .unwrap();
        inventory
            .set_slot(4, Some(ItemStack::new(267, 1, 0).unwrap()))
            .unwrap();
        inventory
            .set_slot(8, Some(ItemStack::new(3, 1, 0).unwrap()))
            .unwrap();
        let (terrain, items) = hotbar_vertices(&inventory, 1280, 720, 3);
        assert_eq!(terrain.len(), 2 * 6);
        assert_eq!(items.len(), 6);
        assert_eq!(terrain[0][2], 16.0 / 256.0);
        assert_eq!(terrain[6][2], 32.0 / 256.0);
        assert_eq!(items[0][2], 32.0 / 256.0);
        assert!(terrain[0][0] < terrain[6][0]);
    }

    #[test]
    fn native_counts_omit_one_and_draw_shadow_then_foreground_for_two_digits() {
        let mut inventory = Inventory::new();
        inventory
            .set_slot(0, Some(ItemStack::new(1, 1, 0).unwrap()))
            .unwrap();
        inventory
            .set_slot(1, Some(ItemStack::new(332, 2, 0).unwrap()))
            .unwrap();
        inventory
            .set_slot(2, Some(ItemStack::new(332, 16, 0).unwrap()))
            .unwrap();
        let counts = count_vertices(&inventory, 1280, 720, 3, true);
        assert_eq!(counts.shadow.len(), 3 * 6);
        assert_eq!(counts.foreground.len(), 3 * 6);
        assert_eq!(counts.foreground[0][3], 24.0 / 128.0);
        assert_eq!(counts.foreground[1][2], 24.0 / 128.0);
        assert_eq!(counts.foreground[2][3], 32.0 / 128.0);
        assert_eq!(counts.foreground[6][2], 8.0 / 128.0);
        assert_eq!(counts.foreground[12][2], 48.0 / 128.0);
        assert!(counts.shadow[0][0] > counts.foreground[0][0]);
        assert!(counts.shadow[0][1] < counts.foreground[0][1]);
    }

    #[test]
    fn inventory_counts_scale_and_use_same_native_anchor() {
        let mut inventory = Inventory::new();
        inventory
            .set_slot(35, Some(ItemStack::new(332, 2, 0).unwrap()))
            .unwrap();
        let counts = count_vertices(&inventory, 640, 480, 2, false);
        assert_eq!(counts.foreground.len(), 6);
        assert_eq!(counts.foreground[0][2], 16.0 / 128.0);
        assert_eq!(counts.foreground[0][3], 24.0 / 128.0);
    }

    #[test]
    fn cursor_clicks_pickup_split_place_and_return_without_changing_selection() {
        let mut screen = InventoryScreen::default();
        let mut inventory = Inventory::new();
        inventory
            .set_slot(0, Some(ItemStack::new(1, 5, 0).unwrap()))
            .unwrap();
        inventory.select_hotbar(4).unwrap();
        assert_eq!(
            screen.key_event(true, false, true, false, true),
            Transition::Opened
        );
        assert!(screen.click(&mut inventory, 0, false));
        assert_eq!(screen.cursor.unwrap().count(), 5);
        assert!(screen.click(&mut inventory, 9, false));
        assert!(screen.click(&mut inventory, 9, true));
        assert_eq!(screen.cursor.unwrap().count(), 3);
        assert_eq!(inventory.stack_at(9).unwrap().unwrap().count(), 2);
        assert!(screen.click(&mut inventory, 9, false));
        assert_eq!(inventory.stack_at(9).unwrap().unwrap().count(), 5);
        screen.return_cursor(&mut inventory);
        assert!(screen.cursor.is_none());
        assert_eq!(inventory.selected_hotbar(), 4);
    }

    #[test]
    fn hitboxes_are_native_eighteen_pixels_and_closed_screen_is_gated() {
        let mut screen = InventoryScreen::default();
        assert_eq!(screen.slot_at(1280, 720, 3, 640.0, 500.0), None);
        let mut inventory = Inventory::new();
        screen.key_event(true, false, true, false, true);
        let layout = native_layout(1280, 720, 3);
        assert_eq!(
            screen.slot_at(1280, 720, 3, layout.slots[0].x - 4.0, layout.slots[0].y),
            None
        );
        assert_eq!(
            screen.slot_at(1280, 720, 3, layout.slots[0].x - 3.0, layout.slots[0].y),
            Some(0)
        );
        assert!(!screen.click(&mut inventory, 36, false));
        assert!(screen.click(&mut inventory, 0, false));
    }

    #[test]
    fn full_inventory_keeps_cursor_and_reopens_screen_on_close() {
        let mut screen = InventoryScreen::default();
        let mut inventory = Inventory::new();
        let full = ItemStack::new(3, 64, 0).unwrap();
        for slot in 0..MAIN_SLOT_COUNT {
            inventory.set_slot(slot, Some(full)).unwrap();
        }
        screen.open = true;
        screen.cursor = Some(ItemStack::new(1, 5, 0).unwrap());
        assert!(!screen.return_cursor(&mut inventory));
        assert!(screen.is_open());
        assert_eq!(screen.cursor.unwrap().count(), 5);
    }

    #[test]
    fn cursor_render_uses_mouse_relative_origin_and_correct_atlas_and_count_pass() {
        let (terrain, items, counts) = cursor_vertices(
            Some(ItemStack::new(1, 2, 0).unwrap()),
            1280,
            720,
            3,
            (700.0, 500.0),
            (600.0, 400.0),
        );
        assert_eq!(terrain.len(), 6);
        assert!(items.is_empty());
        assert_eq!(terrain[0][2], 16.0 / 256.0);
        assert_eq!(terrain[0][0], (76.0 / 1280.0) * 2.0 - 1.0);
        assert_eq!(counts.shadow.len(), 6);
        assert_eq!(counts.foreground.len(), 6);
        let (_, items, counts) = cursor_vertices(
            Some(ItemStack::new(267, 1, 0).unwrap()),
            1280,
            720,
            3,
            (700.0, 500.0),
            (600.0, 400.0),
        );
        assert_eq!(items.len(), 6);
        assert_eq!(counts.foreground.len(), 0);
    }
}
