# World selection C01 native evidence

Source: `.local/minecraft/client-b1.7.3.jar`, SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`.

The audit hashes the obfuscated classes `fu` (main menu), `rq` (world
selection), `mg` (save list), `vb` (save summary), and `tq` (old-format save
backend), then checks their `javap -c -private` output. It covers the
singleplayer transition, sorted save listing, 36-pixel rows, selection and
double-click loading, bottom button labels/IDs/coordinates, Cancel returning
to the parent, and the loading re-entry guard.

The package is deliberately read-only. Create, rename, delete and confirmation
flows (`fj`, `jk`, `qt`) are outside C01; save mutation is excluded and the
test asserts an empty mutation record. No save fixture is required because the
evidence is pinned to the original class bytecode.

Class hashes:

| class | SHA-256 |
| --- | --- |
| `fu.class` | `baa1bc76a2451de306229c87a2964b48abf09c0b16353c25f1bb4cdcff79cb2c` |
| `rq.class` | `7f3fdc2729f5895cf414601075fa847d61bb7a719f73d60a39c69bf97a16c90f` |
| `mg.class` | `0b02e04a20ed97e80a01c8733d13c58937d193a328adaaa568a5e53958b4fe` |
| `vb.class` | `ff8c4833109f9bd4439ebc4ddb718ad303fec9074a2f66a744f06de754d169a9` |
| `tq.class` | `339175cb6bdd4f97024f30c9c501b4efe9221f60672a9fb3b017890b5b2272c8` |
