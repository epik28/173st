# Native durability registry

The oracle checks the pinned client JAR SHA-256 and class code sources, then
records `iz.j()` maximum damage and checks every icon index `iz.b()` from zero
through that maximum for each item ID 256–359. A maximum of zero here does not
establish valid metadata variants. Runtime support is restricted to durable
items with damage at most the native maximum; non-durable metadata stays zero.
The icon fixture covers atlas indices, not durability bars or rendering pixels.

```sh
mkdir -p /tmp/inventory-damage-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/inventory-damage-classes tools/oracle/src/InventoryDamageOracle.java
java -cp /tmp/inventory-damage-classes:../.local/minecraft/client-b1.7.3.jar InventoryDamageOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/inventory-damage.tsv
diff -u tests/fixtures/inventory_damage/native.tsv /tmp/inventory-damage.tsv
cargo test --locked --offline -p beta-core durability_limits_match_native_registry_fixture
cargo test --locked --offline -p beta-viewer durable_item_icons_match_native_damage_range
```
