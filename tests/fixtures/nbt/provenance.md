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

C03 extends the generated live corpus to 5250 trees: signed endpoints for all
numeric widths, raw IEEE boundaries/NaN payloads, modified UTF-8 including a
65535-byte string, empty containers, duplicate names, HashMap resize/collision,
nested lists/compounds and depth 64. The 12 shared invalid inputs are run
individually; a combined failing stream alone does not prove their rejection.
The former `negative-list` fixture was actually an End followed by trailing
bytes; its ID is corrected to `compound-end-trailer`, and a real negative-list
length is measured separately in the native-differences checks.

`javap -private -c` of original `ij`, `nu`, `sp` confirms the following, also
measured live by `verify_report.py --protocol`:

- `nu` stores fields in `java.util.HashMap`; duplicate names replace the value.
- `DataOutput.writeFloat/writeDouble` canonicalize NaN payloads. The independent
  wire inspection retains raw input bits and explicitly checks this write rule.
- `sp` writes every empty list with element type Byte, even if a different type
  was read. Rust retains the declared type as required by the wire/API contract.
- `sp` accepts a negative list length as empty, and can read nonempty End lists
  or an unknown element type when length is zero. Rust rejects these inputs.
- `DataInput.readUTF` accepts isolated UTF-16 surrogates; Rust `String` cannot
  represent them, so the Rust API rejects them explicitly.

These differences are preserved as separate evidence, not normalized away or
counted among the 5250 equal-output live cases. Root-End/trailing-byte and TSV
guards belong to our wrapper, not to the original NBT implementation. Java is
not used to validate our resource limits: hostile lengths are sent only to the
safe Rust reader. IntArray is wire-format evidence, not original-Beta parity.
