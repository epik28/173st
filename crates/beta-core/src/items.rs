//! Bounded, in-memory model of the native-confirmed Beta 1.7.3 main inventory.
//!
//! Stack limits come from the pinned client registry. This model performs no I/O
//! and does not persist state.

use std::{error::Error, fmt};

pub const MAIN_SLOT_COUNT: usize = 36;
pub const HOTBAR_SLOT_COUNT: usize = 9;

/// An item identity whose stack limit was captured from the pinned client.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemKind {
    id: u16,
    max_stack: u8,
}

impl ItemKind {
    pub fn from_native_id(id: u16) -> Result<Self, ItemError> {
        let max_stack = match id {
            1 | 3 | 17 | 35 => 64, // stone, dirt, logs, wool
            // Queried through iz.c() in the pinned Beta 1.7.3 client.
            256..=359 => match id {
                262..=266
                | 280..=281
                | 287..=289
                | 295..=296
                | 318
                | 321
                | 331
                | 334
                | 336..=341
                | 345
                | 347..=348
                | 351..=353
                | 356 => 64,
                332 | 344 => 16,
                357 => 8,
                _ => 1,
            },
            other => return Err(ItemError::UnsupportedItem(other)),
        };
        Ok(Self { id, max_stack })
    }

    /// Maximum native durability damage, zero for non-durable items.
    pub const fn max_damage(self) -> u16 {
        match self.id {
            256..=258 | 267 | 292 => 250,
            259 | 346 => 64,
            268..=271 | 290 => 59,
            272..=275 | 291 => 131,
            276..=279 | 293 => 1561,
            283..=286 | 294 => 32,
            298 => 33,
            299 => 48,
            300 => 45,
            301 => 39,
            302 => 66,
            303 => 96,
            304 => 90,
            305 => 78,
            306 => 132,
            307 => 192,
            308 => 180,
            309 => 156,
            310 => 264,
            311 => 384,
            312 => 360,
            313 => 312,
            314 => 66,
            315 => 96,
            316 => 90,
            317 => 78,
            359 => 238,
            _ => 0,
        }
    }

    pub const fn id(self) -> u16 {
        self.id
    }

    pub const fn max_stack(self) -> u8 {
        self.max_stack
    }

    /// Largest supported stack data value; variants are not durability.
    pub const fn max_stack_data(self) -> u16 {
        match self.id {
            17 | 35 | 351 => 15,
            263 => 1,
            _ => self.max_damage(),
        }
    }
}

/// A supported native item stack with bounded durable-item damage.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ItemStack {
    kind: ItemKind,
    count: u8,
    damage: u16,
}

impl ItemStack {
    pub fn new(id: u16, count: u8, damage: u16) -> Result<Self, ItemError> {
        let kind = ItemKind::from_native_id(id)?;
        if damage > kind.max_stack_data() {
            return Err(ItemError::UnsupportedDamage(damage));
        }
        if count == 0 || count > kind.max_stack {
            return Err(ItemError::InvalidCount {
                count,
                max: kind.max_stack,
            });
        }
        Ok(Self {
            kind,
            count,
            damage,
        })
    }

    pub const fn id(self) -> u16 {
        self.kind.id
    }

    pub const fn count(self) -> u8 {
        self.count
    }

    pub const fn damage(self) -> u16 {
        self.damage
    }

    pub const fn max_stack(self) -> u8 {
        self.kind.max_stack
    }

    /// Native block face tiles for the supported log and wool item variants.
    /// The caller chooses how to project the six faces into an inventory icon.
    pub fn variant_block_face_tiles(self) -> Option<[u16; 6]> {
        match self.id() {
            17 => {
                let side = match self.damage {
                    1 => 116,
                    2 => 117,
                    _ => 20,
                };
                Some([21, 21, side, side, side, side])
            }
            35 => {
                let tile = [
                    64, 210, 194, 178, 162, 146, 130, 114, 225, 209, 193, 177, 161, 145, 129, 113,
                ][usize::from(self.damage)];
                Some([tile; 6])
            }
            _ => None,
        }
    }

    /// Split `amount` items off this stack, preserving the receiver on error.
    pub fn split(&mut self, amount: u8) -> Result<Self, ItemError> {
        if amount == 0 || amount > self.count {
            return Err(ItemError::InvalidSplit {
                amount,
                count: self.count,
            });
        }
        self.count -= amount;
        Ok(Self {
            count: amount,
            ..*self
        })
    }

    /// Remove one item if the requested native item ID matches.
    pub fn remove_one_by_id(&mut self, id: u16) -> bool {
        if self.id() != id {
            return false;
        }
        self.count -= 1;
        true
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ItemError {
    UnsupportedItem(u16),
    UnsupportedDamage(u16),
    InvalidCount { count: u8, max: u8 },
    InvalidSplit { amount: u8, count: u8 },
    InvalidSlot(usize),
    InvalidHotbarIndex(i32),
}

impl fmt::Display for ItemError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedItem(id) => write!(f, "item ID {id} is outside the verified set"),
            Self::UnsupportedDamage(damage) => {
                write!(f, "damage value {damage} is outside the verified set")
            }
            Self::InvalidCount { count, max } => {
                write!(f, "stack count {count} is invalid for maximum {max}")
            }
            Self::InvalidSplit { amount, count } => {
                write!(f, "cannot split {amount} items from stack of {count}")
            }
            Self::InvalidSlot(slot) => write!(f, "main inventory slot {slot} is out of range"),
            Self::InvalidHotbarIndex(index) => {
                write!(f, "hotbar index {index} is out of range")
            }
        }
    }
}

impl Error for ItemError {}

/// The 36 main inventory slots and selected hotbar slot. Empty stacks are
/// represented by `None`; insertion is all-or-nothing when total capacity is
/// insufficient, matching the captured fully occupied failure case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Inventory {
    slots: [Option<ItemStack>; MAIN_SLOT_COUNT],
    selected: u8,
}

impl Default for Inventory {
    fn default() -> Self {
        Self::new()
    }
}

impl Inventory {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAIN_SLOT_COUNT],
            selected: 0,
        }
    }

    pub const fn selected_hotbar(&self) -> u8 {
        self.selected
    }

    pub fn select_hotbar(&mut self, index: i32) -> Result<(), ItemError> {
        if !(0..HOTBAR_SLOT_COUNT as i32).contains(&index) {
            return Err(ItemError::InvalidHotbarIndex(index));
        }
        self.selected = index as u8;
        Ok(())
    }

    pub fn selected_stack(&self) -> Option<ItemStack> {
        self.slots[usize::from(self.selected)]
    }

    pub fn stack_at(&self, slot: usize) -> Result<Option<ItemStack>, ItemError> {
        self.slots
            .get(slot)
            .copied()
            .ok_or(ItemError::InvalidSlot(slot))
    }

    pub fn set_slot(
        &mut self,
        slot: usize,
        stack: Option<ItemStack>,
    ) -> Result<Option<ItemStack>, ItemError> {
        let target = self
            .slots
            .get_mut(slot)
            .ok_or(ItemError::InvalidSlot(slot))?;
        Ok(std::mem::replace(target, stack))
    }

    pub fn split_slot(&mut self, slot: usize, amount: u8) -> Result<ItemStack, ItemError> {
        let target = self
            .slots
            .get_mut(slot)
            .ok_or(ItemError::InvalidSlot(slot))?;
        let stack = target
            .as_mut()
            .ok_or(ItemError::InvalidSplit { amount, count: 0 })?;
        let split = stack.split(amount)?;
        if stack.count == 0 {
            *target = None;
        }
        Ok(split)
    }

    pub fn remove_one_by_id(&mut self, id: u16) -> bool {
        for slot in &mut self.slots {
            if let Some(stack) = slot.as_mut()
                && stack.remove_one_by_id(id)
            {
                if stack.count == 0 {
                    *slot = None;
                }
                return true;
            }
        }
        false
    }

    /// Merge compatible stacks first, then fill empty slots in ascending slot
    /// order. If there is not enough total capacity, neither side is changed.
    pub fn insert(&mut self, incoming: &mut Option<ItemStack>) -> Result<bool, ItemError> {
        let Some(stack) = *incoming else {
            return Ok(true);
        };
        if stack.count == 0 || stack.count > stack.max_stack() {
            return Err(ItemError::InvalidCount {
                count: stack.count,
                max: stack.max_stack(),
            });
        }

        let capacity: u16 = self
            .slots
            .iter()
            .map(|slot| match slot {
                None => u16::from(stack.max_stack()),
                Some(existing)
                    if existing.kind == stack.kind && existing.damage == stack.damage =>
                {
                    u16::from(existing.max_stack() - existing.count)
                }
                Some(_) => 0,
            })
            .sum();
        if capacity < u16::from(stack.count) {
            return Ok(false);
        }

        let mut remaining = stack.count;
        let mut next_slots = self.slots;
        for slot in &mut next_slots {
            if remaining == 0 {
                break;
            }
            if let Some(existing) = slot.as_mut()
                && existing.kind == stack.kind
                && existing.damage == stack.damage
            {
                let moved = remaining.min(existing.max_stack() - existing.count);
                existing.count += moved;
                remaining -= moved;
            }
        }
        for slot in &mut next_slots {
            if remaining == 0 {
                break;
            }
            if slot.is_none() {
                let moved = remaining.min(stack.max_stack());
                *slot = Some(ItemStack {
                    count: moved,
                    ..stack
                });
                remaining -= moved;
            }
        }

        debug_assert_eq!(remaining, 0);
        self.slots = next_slots;
        *incoming = None;
        Ok(true)
    }

    /// Select an item in the hotbar by ID, preferring the current selection and
    /// then the first matching slot in native inventory order.
    pub fn select_item_id(&mut self, id: u16) -> bool {
        if self.slots[..HOTBAR_SLOT_COUNT]
            .get(usize::from(self.selected))
            .and_then(Option::as_ref)
            .is_some_and(|stack| stack.id() == id)
        {
            return true;
        }
        if let Some(index) = self.slots[..HOTBAR_SLOT_COUNT]
            .iter()
            .position(|slot| slot.is_some_and(|stack| stack.id() == id))
        {
            self.selected = index as u8;
            true
        } else {
            false
        }
    }

    /// Native scroll behavior: positive steps move toward lower slot indexes,
    /// with wraparound in the nine-slot hotbar.
    pub fn cycle_hotbar(&mut self, steps: i32) {
        let index = (i32::from(self.selected) - steps).rem_euclid(HOTBAR_SLOT_COUNT as i32);
        self.selected = index as u8;
    }

    /// Native invalid selected indexes return no selected stack. This checked
    /// method also lets callers test raw index bounds without mutating state.
    pub fn stack_at_hotbar_index(&self, index: i32) -> Result<Option<ItemStack>, ItemError> {
        if !(0..HOTBAR_SLOT_COUNT as i32).contains(&index) {
            return Err(ItemError::InvalidHotbarIndex(index));
        }
        Ok(self.slots[index as usize])
    }

    /// Return the selected stack result for a raw native index without storing
    /// that index. Like the captured native lookup, an out-of-range index has
    /// no selected stack; the safe inventory selection remains within 0..=8.
    pub fn selected_stack_at(&self, index: i32) -> Option<ItemStack> {
        if !(0..HOTBAR_SLOT_COUNT as i32).contains(&index) {
            return None;
        }
        self.slots[index as usize]
    }

    pub fn slots(&self) -> &[Option<ItemStack>; MAIN_SLOT_COUNT] {
        &self.slots
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_log_and_wool_stack_variants_match_face_texture_fixture() {
        let rows: Vec<_> = include_str!("../../../tests/fixtures/inventory_variants/native.tsv")
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(rows.len(), 32);
        for row in rows {
            let fields: Vec<_> = row.split('\t').collect();
            assert_eq!(fields.len(), 5);
            let id = fields[0].parse().unwrap();
            let data = fields[1].parse().unwrap();
            let stack = ItemStack::new(id, 1, data).unwrap();
            assert_eq!(stack.max_stack().to_string(), fields[2]);
            assert_eq!(
                ItemKind::from_native_id(id)
                    .unwrap()
                    .max_damage()
                    .to_string(),
                fields[3]
            );
            assert_eq!(
                stack
                    .variant_block_face_tiles()
                    .unwrap()
                    .map(|tile| tile.to_string())
                    .join(","),
                fields[4]
            );
        }
        for id in [17, 35] {
            assert!(ItemStack::new(id, 1, 16).is_err());
            assert_eq!(ItemKind::from_native_id(id).unwrap().max_damage(), 0);
        }
        assert_eq!(
            ItemStack::new(1, 1, 0).unwrap().variant_block_face_tiles(),
            None
        );
    }
    use std::collections::BTreeMap;

    type NativeState = (i32, Option<ItemStack>, Vec<Option<ItemStack>>);
    type NativeStates = BTreeMap<(String, String), NativeState>;

    fn stack(id: u16, count: u8) -> ItemStack {
        ItemStack::new(id, count, 0).unwrap()
    }

    fn native_states() -> NativeStates {
        let text = include_str!("../../../tests/fixtures/items/native.tsv");
        let mut states = BTreeMap::new();
        for line in text.lines() {
            let cols: Vec<_> = line.split('\t').collect();
            assert_eq!(cols.len(), 6, "bad native row {line}");
            let selected = cols[3].parse().unwrap();
            let selected_stack = parse_stack(cols[4]);
            let slots: Vec<_> = cols[5].split('|').map(parse_stack).collect();
            assert_eq!(slots.len(), MAIN_SLOT_COUNT, "bad slot count in {line}");
            states.insert(
                (cols[0].to_owned(), cols[1].to_owned()),
                (selected, selected_stack, slots),
            );
        }
        states
    }

    fn parse_stack(value: &str) -> Option<ItemStack> {
        if value == "-" {
            return None;
        }
        let fields: Vec<_> = value.split(',').collect();
        assert!(fields.len() >= 4, "bad stack {value}");
        let id = fields[0].parse().unwrap();
        let count = fields[1].parse().unwrap();
        let damage = fields[2].parse().unwrap();
        let max: u8 = fields[3].parse().unwrap();
        let stack = ItemStack::new(id, count, damage).unwrap();
        assert_eq!(stack.max_stack(), max, "native max mismatch for {value}");
        Some(stack)
    }

    fn assert_native(states: &NativeStates, scenario: &str, state: &str, inventory: &Inventory) {
        let (selected, selected_stack, slots) = states
            .get(&(scenario.to_owned(), state.to_owned()))
            .unwrap_or_else(|| panic!("missing native state {scenario}/{state}"));
        if (0..HOTBAR_SLOT_COUNT as i32).contains(selected) {
            assert_eq!(inventory.selected_hotbar(), *selected as u8);
            assert_eq!(inventory.selected_stack(), *selected_stack);
        } else {
            assert_eq!(inventory.selected_stack_at(*selected), *selected_stack);
            assert!(usize::from(inventory.selected_hotbar()) < HOTBAR_SLOT_COUNT);
        }
        assert_eq!(inventory.slots().as_slice(), slots.as_slice());
    }

    #[test]
    fn coal_and_charcoal_metadata_match_native_insertion() {
        let mut cases = 0;
        for row in include_str!("../../../tests/fixtures/item_variants/native.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<&str> = row.split('\t').collect();
            if fields[0] != "263" {
                continue;
            }
            let damage: u16 = fields[1].parse().unwrap();
            if damage > 1 {
                continue;
            }
            assert_eq!(fields[2], "7");
            assert_eq!(fields[3], "true");
            assert_eq!(
                fields[8],
                if damage == 0 {
                    "item.coal"
                } else {
                    "item.charcoal"
                }
            );
            let mut inventory = Inventory::new();
            inventory
                .set_slot(0, Some(ItemStack::new(263, 1, 0).unwrap()))
                .unwrap();
            let mut incoming = Some(ItemStack::new(263, 1, damage).unwrap());
            assert!(inventory.insert(&mut incoming).unwrap());
            assert_eq!(
                inventory.stack_at(0).unwrap().unwrap().count(),
                fields[6].parse::<u8>().unwrap()
            );
            assert_eq!(
                inventory
                    .stack_at(1)
                    .unwrap()
                    .map_or(0, |stack| stack.damage() + 1),
                fields[7].parse::<u16>().unwrap()
            );
            cases += 1;
        }
        assert_eq!(cases, 2);
        assert!(ItemStack::new(263, 1, 2).is_err());
        assert_eq!(ItemKind::from_native_id(263).unwrap().max_damage(), 0);
    }

    #[test]
    fn dye_metadata_matches_native_merges() {
        let mut icon_rows = 0;
        let mut merge_rows = 0;
        for row in include_str!("../../../tests/fixtures/dye_inventory/native.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<&str> = row.split('\t').collect();
            let damage: u16 = fields[1].parse().unwrap();
            if fields[0] == "icon" {
                let stack = ItemStack::new(351, 1, damage).unwrap();
                assert_eq!(stack.damage(), damage);
                assert_eq!(stack.max_stack(), 64);
                icon_rows += 1;
                continue;
            }
            let other: u16 = fields[2].parse().unwrap();
            let mut inv = Inventory::new();
            inv.set_slot(0, Some(ItemStack::new(351, 10, damage).unwrap()))
                .unwrap();
            let mut incoming = Some(ItemStack::new(351, 5, other).unwrap());
            assert!(inv.insert(&mut incoming).unwrap());
            assert_eq!(
                inv.stack_at(0).unwrap().unwrap().count(),
                fields[3].parse::<u8>().unwrap()
            );
            let second = inv.stack_at(1).unwrap().map_or(0, |s| s.damage() + 1);
            assert_eq!(second, fields[4].parse::<u16>().unwrap());
            merge_rows += 1;
        }
        assert_eq!((icon_rows, merge_rows), (16, 256));
        assert!(ItemStack::new(351, 1, 16).is_err());
        assert_eq!(ItemKind::from_native_id(351).unwrap().max_damage(), 0);
    }

    #[test]
    fn durability_limits_match_native_registry_fixture() {
        for row in include_str!("../../../tests/fixtures/inventory_damage/native.tsv")
            .lines()
            .skip(1)
        {
            let fields: Vec<&str> = row.split('\t').collect();
            let id: u16 = fields[0].parse().unwrap();
            let max: u16 = fields[1].parse().unwrap();
            assert_eq!(
                ItemKind::from_native_id(id).unwrap().max_damage(),
                max,
                "ID {id}"
            );
            assert!(ItemStack::new(id, 1, max).is_ok());
            if max > 0 {
                assert!(ItemStack::new(id, 1, max + 1).is_err());
            }
        }
    }

    #[test]
    fn stack_limits_match_native_registry_fixture() {
        let rows = include_str!("../../../tests/fixtures/inventory_registry/native.tsv");
        for row in rows.lines().skip(1) {
            let values: Vec<u16> = row.split('\t').map(|s| s.parse().unwrap()).collect();
            assert_eq!(
                u16::from(ItemKind::from_native_id(values[0]).unwrap().max_stack()),
                values[4],
                "ID {}",
                values[0]
            );
        }
    }

    #[test]
    fn additional_native_stack_limits_are_enforced() {
        for (id, max) in [
            (256, 1),
            (262, 64),
            (332, 16),
            (344, 16),
            (357, 8),
            (359, 1),
        ] {
            assert!(ItemStack::new(id, max, 0).is_ok());
            assert!(ItemStack::new(id, max + 1, 0).is_err());
        }
    }

    #[test]
    fn split_remove_and_replace_match_every_native_state() {
        let states = native_states();
        let mut inventory = Inventory::new();
        assert_native(&states, "split_remove_replace", "empty", &inventory);
        inventory.set_slot(0, Some(stack(1, 10))).unwrap();
        assert_native(&states, "split_remove_replace", "set", &inventory);
        assert_eq!(inventory.split_slot(0, 4).unwrap(), stack(1, 4));
        assert_native(&states, "split_remove_replace", "split", &inventory);
        assert!(inventory.remove_one_by_id(1));
        assert_native(
            &states,
            "split_remove_replace",
            "remove_one_by_id",
            &inventory,
        );
        inventory.set_slot(0, Some(stack(3, 2))).unwrap();
        assert_native(&states, "split_remove_replace", "replace", &inventory);
    }

    #[test]
    fn merge_and_empty_slot_spill_match_native_rows() {
        let states = native_states();
        let mut inventory = Inventory::new();
        inventory.set_slot(0, Some(stack(1, 50))).unwrap();
        assert_native(&states, "merge", "before", &inventory);
        let mut incoming = Some(stack(1, 20));
        assert!(inventory.insert(&mut incoming).unwrap());
        assert_eq!(incoming, None);
        assert_native(&states, "merge", "after", &inventory);
    }

    #[test]
    fn insufficient_capacity_is_atomic_and_limits_match_native() {
        let states = native_states();
        let mut inventory = Inventory::new();
        for slot in 0..MAIN_SLOT_COUNT {
            inventory.set_slot(slot, Some(stack(1, 64))).unwrap();
        }
        assert_native(&states, "overflow", "before", &inventory);
        let before = inventory.clone();
        let mut incoming = Some(stack(3, 1));
        assert!(!inventory.insert(&mut incoming).unwrap());
        assert_eq!(inventory, before);
        assert_eq!(incoming, Some(stack(3, 1)));
        assert_native(&states, "overflow", "after", &inventory);

        for (id, max) in [(1, 64), (3, 64), (332, 16), (267, 1)] {
            assert_eq!(ItemKind::from_native_id(id).unwrap().max_stack(), max);
        }
        assert_eq!(
            ItemStack::new(332, 17, 0),
            Err(ItemError::InvalidCount { count: 17, max: 16 })
        );
        assert_eq!(
            ItemStack::new(999, 1, 0),
            Err(ItemError::UnsupportedItem(999))
        );
        assert_eq!(
            ItemStack::new(1, 1, 1),
            Err(ItemError::UnsupportedDamage(1))
        );
    }

    #[test]
    fn smaller_limit_and_non_stackable_insert_match_native_rows() {
        let states = native_states();
        for (scenario, id, initial, added, expected) in [
            ("smaller_limit", 332, 15, 3, 16),
            ("non_stackable", 267, 1, 1, 1),
        ] {
            let mut inventory = Inventory::new();
            inventory.set_slot(0, Some(stack(id, initial))).unwrap();
            assert_native(&states, scenario, "before", &inventory);
            let mut incoming = Some(stack(id, added));
            assert!(inventory.insert(&mut incoming).unwrap());
            assert_eq!(inventory.stack_at(0).unwrap().unwrap().count(), expected);
            assert_native(&states, scenario, "after", &inventory);
        }
    }

    #[test]
    fn hotbar_selection_and_scroll_match_native_rows() {
        let states = native_states();
        let mut inventory = Inventory::new();
        inventory.set_slot(0, Some(stack(1, 2))).unwrap();
        inventory.set_slot(8, Some(stack(3, 4))).unwrap();
        assert_native(&states, "hotbar_bounds", "initial", &inventory);
        inventory.cycle_hotbar(1);
        assert_native(&states, "hotbar_bounds", "cycle_positive", &inventory);
        inventory.cycle_hotbar(-1);
        assert_native(&states, "hotbar_bounds", "cycle_negative", &inventory);
        inventory.cycle_hotbar(9);
        assert_native(&states, "hotbar_bounds", "cycle_wrap", &inventory);
        assert_native(&states, "hotbar_bounds", "index_minus_one", &inventory);
        assert_native(&states, "hotbar_bounds", "index_nine", &inventory);
        assert_eq!(inventory.selected_stack_at(-1), None);
        assert_eq!(inventory.selected_stack_at(9), None);

        inventory.select_hotbar(8).unwrap();
        assert!(inventory.select_item_id(3));
        assert_native(&states, "hotbar_bounds", "select_id_3", &inventory);
        assert!(!inventory.select_item_id(999));
        assert_native(&states, "hotbar_bounds", "select_missing_id", &inventory);
        assert_eq!(
            inventory.stack_at_hotbar_index(-1),
            Err(ItemError::InvalidHotbarIndex(-1))
        );
        assert_eq!(
            inventory.stack_at_hotbar_index(9),
            Err(ItemError::InvalidHotbarIndex(9))
        );
        let before = inventory.clone();
        assert_eq!(
            inventory.select_hotbar(-1),
            Err(ItemError::InvalidHotbarIndex(-1))
        );
        assert_eq!(
            inventory.select_hotbar(9),
            Err(ItemError::InvalidHotbarIndex(9))
        );
        assert_eq!(inventory, before);
    }

    #[test]
    fn invalid_mutations_leave_state_unchanged() {
        let mut inventory = Inventory::new();
        inventory.set_slot(0, Some(stack(1, 5))).unwrap();
        let before = inventory.clone();
        assert_eq!(
            inventory.split_slot(0, 6),
            Err(ItemError::InvalidSplit {
                amount: 6,
                count: 5
            })
        );
        assert_eq!(inventory, before);
        assert_eq!(
            inventory.set_slot(MAIN_SLOT_COUNT, None),
            Err(ItemError::InvalidSlot(MAIN_SLOT_COUNT))
        );
        assert_eq!(inventory, before);
        let mut item = stack(1, 5);
        assert_eq!(
            item.split(0),
            Err(ItemError::InvalidSplit {
                amount: 0,
                count: 5
            })
        );
        assert_eq!(item, stack(1, 5));
        assert!(!inventory.remove_one_by_id(3));
        assert_eq!(inventory, before);
    }
}
