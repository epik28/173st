# ITEMS-001 — first inventory data slice

## Goal and limits

Establish native-backed item-stack and player main-inventory behavior as the
foundation for a later visible inventory. This package covers the 36 main slots,
the nine-slot hotbar and selected hotbar index, stack split/merge/insert, and
slot replacement. It does not add GUI, armor slots, crafting, pickup/drop
entities, tools/durability rules, survival or save writing.

Use only simple native item cases whose IDs, stack limits, and damage behavior
are captured from the pinned Beta 1.7.3 client. General item catalog/metadata and
arbitrary custom NBT tags are outside the initial Rust model; unsupported tagged
or special stacks must fail or remain explicitly unsupported, never be silently
flattened.

## Native evidence

C01 must inspect the pinned JAR and invoke the actual original ItemStack and
inventory methods in isolated in-memory objects. Capture construction, empty
slots, selected hotbar bounds, splitting/removing a stack, merging compatible
stacks up to the native limit, filling an empty slot, rejecting an incompatible
stack, and any native NBT fields required to preserve supported stacks. Use at
least one 64-stack item, one item with a smaller native stack limit, and one
non-stackable item if they can be initialized safely. Record exact class/method
hashes, JAR code source, before/after slots, and two identical native runs.
Do not initialize or write the user's changed Prism save.

## Rust and probe

Implement a bounded pure `beta_core::items` model for 36 main slots, hotbar
selection 0..8, item ID, count, damage, and an explicit verified stack limit.
Follow C01 for actual edge behavior. Mutations must be atomic on invalid inputs;
merging must preserve item identity and respect native max count. Keep item
catalog lookup outside this first model. Add a bounded deterministic TSV probe
after the strict runner contract is committed. Malformed/duplicate/over-limit
input fails without partial stdout.

## Acceptance

- C01 captures and audits real original calls, with repeat-identical evidence.
- C02 runner compares ordered typed slot/state results exactly and rejects
  missing/extra/reordered rows, wrong types, limits, and changed raw values.
- A01/A02 match every supported native case and have meaningful atomicity,
  bounds, merge/split, and hotbar tests.
- Full workspace checks and all existing strict parity suites pass.
- This is an inventory data API only; no visible inventory or complete gameplay
  claim is made. Do not change readiness percentage for oracle infrastructure
  alone.

## Tasks

- C01 native ItemStack/inventory behavior corpus.
- C02 strict inventory runner and retained evidence review.
- A01 Rust stack and main-inventory model.
- A02 bounded probe and fixture-driven parity.
- C03 integration and acceptance.
