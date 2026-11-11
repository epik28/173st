# Native inventory registry

`native.tsv` was produced from the pinned Beta 1.7.3 client. The oracle checks
SHA-256 of the full JAR, the `iz`, `gm`, and `uu` classes, and their load source.
Each row records `iz.b()` at damage 0, 1 and 15 and `iz.c()` at damage 0.
The two nonzero damage samples identify variants but are not an exhaustive
proof that all other damage values behave identically.

From the repository root (JDK 17 or later):

```sh
mkdir -p /tmp/inventory-registry-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/inventory-registry-classes tools/oracle/src/InventoryRegistryOracle.java
java -cp /tmp/inventory-registry-classes:../.local/minecraft/client-b1.7.3.jar InventoryRegistryOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/inventory-registry.tsv
diff -u tests/fixtures/inventory_registry/native.tsv /tmp/inventory-registry.tsv
cargo test --locked --offline -p beta-core stack_limits_match_native_registry_fixture
cargo test --locked --offline -p beta-viewer item_tiles_match_native_registry_fixture
```

The viewer rejects IDs 345 and 347 because their compass and clock atlas tiles
are updated according to world context, and ID 351 because its icon depends on
damage. All remaining accepted icons are limited to damage zero. The corpus
captures registry indices, not rendered pixels or animation behavior.
