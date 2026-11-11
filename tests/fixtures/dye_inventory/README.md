# Native dye inventory

The oracle checks the pinned JAR SHA-256 and code sources, then queries all 16
native dye icon indices and executes all 256 pairs of metadata values through
`ix.a(iz)` with an existing stack of 10 and incoming stack of 5. In merge rows,
`value0` is the count left in slot 0; `value1` is zero for an empty slot 1,
otherwise one plus its metadata, allowing metadata zero to be distinguished.
The corpus records in-memory native behavior, without an on-disk world.

```sh
mkdir -p /tmp/dye-inventory-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/dye-inventory-classes tools/oracle/src/DyeInventoryOracle.java
java -cp /tmp/dye-inventory-classes:../.local/minecraft/client-b1.7.3.jar DyeInventoryOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/dye-inventory.tsv
diff -u tests/fixtures/dye_inventory/native.tsv /tmp/dye-inventory.tsv
cargo test --locked --offline -p beta-core dye_metadata_matches_native_merges
```
