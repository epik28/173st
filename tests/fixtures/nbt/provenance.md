# NBT corpus provenance

The reference is the original Beta 1.7.3 client JAR at
`../.local/minecraft/client-b1.7.3.jar` (SHA-256
`af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c`).
The adapter invokes the original obfuscated classes `ij` (tag factory and
named-tag I/O) and `nu` (compound root), via `ij.b(DataInput)` and
`ij.a(ij,DataOutput)`. The corpus is a wire-format audit only; it is not a
Rust parity result and does not include gzip or region containers.

The original Beta factory only exposes tag ids 0 through 10; `TAG_Int_Array`
(11) is retained in `wire-only-int-array.tsv` as explicitly labelled format
evidence and is expected to be rejected by this original API.
