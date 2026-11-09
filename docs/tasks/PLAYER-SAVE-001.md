# PLAYER-SAVE-001 — read supported inventory from the saved player

## Goal

Populate the existing read-only inventory screen from the `Player.Inventory`
compound list already present in the hash-pinned `level.dat` copy.

## Contract

- Read only the already validated copied `level.dat`; never reopen or modify the
  live save. Keep the historic NBT and world fixtures unchanged.
- Parse Beta player stack fields `Slot` (byte), `id` (short), `Count` (byte),
  and `Damage` (short). Only slots 0–35 and item stacks accepted by the existing
  `beta_core::items::ItemStack` model are displayed.
- Unsupported IDs, damage, malformed fields and invalid counts are skipped and
  counted explicitly. Armor slots and other player fields are outside this card.
- The E inventory screen displays supported saved stacks through its existing
  atlas path. This remains read-only: no item movement, counts, survival tick,
  health update, or save operation is added.
- Tests cover empty/missing player data, valid stacks, unsupported stacks,
  malformed data and out-of-range slots.

## Acceptance boundary

This enables a limited view of supported stacks stored in the current save. It
does not claim full item coverage or gameplay parity. Player health and survival
behavior remain future work.
