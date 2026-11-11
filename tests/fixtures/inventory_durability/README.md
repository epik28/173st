# Native durability overlay

The oracle invokes the original pinned `bb.b(sj,ji,iz,int,int)` method with
native stacks at icon origin (10,20). It verifies the client JAR SHA-256 and
native class code sources. The renderer is allocated without its constructor;
the classpath substitutes a recording `nw` tessellator and no-op GL11 state
calls, so the TSV captures the original method's ordered rectangles and RGB
without an OpenGL context. No pixels, blending, or GPU rendering are tested.
The four items sample zero, low, quarter, half, near-limit and limit damage.

```sh
mkdir -p /tmp/inventory-durability-classes
javac -cp ../.local/minecraft/client-b1.7.3.jar -d /tmp/inventory-durability-classes tools/oracle/durability_stub/nw.java tools/oracle/durability_stub/org/lwjgl/opengl/GL11.java tools/oracle/src/InventoryDurabilityOracle.java
java -cp /tmp/inventory-durability-classes:../.local/minecraft/client-b1.7.3.jar InventoryDurabilityOracle ../.local/minecraft/client-b1.7.3.jar > /tmp/inventory-durability.tsv
diff -u tests/fixtures/inventory_durability/native.tsv /tmp/inventory-durability.tsv
cargo test --locked --offline -p beta-viewer durability_quads_match_native_tessellator_capture
```
