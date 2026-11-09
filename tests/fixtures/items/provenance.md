# ITEMS-001 native inventory evidence

The oracle ran on the Beta 1.7.3 client JAR at `../.local/minecraft/client-b1.7.3.jar`,
SHA-256 `af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.
The Java runtime loaded native `iz`, `ix`, `gm`, `uu`, `gs`, and `jl` from that
JAR code source; class SHA-256 values, exact code source, method list, call counts,
input/output hashes, and row count are in `audit.json`. No JAR, extracted class,
decompilation, Prism save, player, World, window, or mutable game resource is in
the fixture or Git.

`iz` is the native stack class: the adapter invokes constructor `iz(int,int,int)`,
split `iz.a(int)`, stack limit `iz.c()`, and damage getter `iz.h()`. `ix` is the
native inventory: it allocates its 36-slot main array and four armor slots using
`ix(gs)` with a null owner, then invokes `ix.a(int,iz)` for slot assignment,
`ix.a(int,int)` for slot splitting/removal, `ix.c(int)` for item-ID decrement,
`ix.a(iz)` for insertion/merge, `ix.b(int)` for hotbar cycling, `ix.b()` for
selected-stack lookup, and `ix.a(int,boolean)` for selecting a matching item ID.

The original item registry needs normal class initialization before the first
stack lookup. The adapter calls `Class.forName("uu")` first. Calling an item
stack method before that bootstrap caused the original static registry to enter
a circular initialization path; the captured bootstrap then completed normally
(`16 achievements`, `151 recipes`) without constructing a world or player.
Only local objects were mutated.

`input.tsv` names six bounded scenarios. `native.tsv` is the ordered output from
two identical direct JAR runs, including every one of the 36 main slots at each
state. Empty slots are `-`; non-empty slots are `id,count,damage,native-limit,
native-item-name`. The pinned IDs used are stone 1 (limit 64), snowball 332
(limit 16), iron sword 267 (limit 1), and dirt 3 for slot replacement. Native
observations include merge overflow spilling into the next empty slot, failed
insertion preserving its input when all 36 slots are occupied, and invalid
selected indices `-1`/`9` returning no selected stack. These are captures from
the original methods, not modeled expected formulas.

The registry bootstrap writes its original `16 achievements` and `151 recipes`
messages to stdout; the adapter captures these separately and records their
SHA-256 in `audit.json`, leaving the ordered TSV free of bootstrap chatter.

Reproduce with `python3 tools/oracle/items_corpus.py --write` (writes fixtures
only after two equal native runs), then verify with `python3 tools/oracle/items_corpus.py`.
