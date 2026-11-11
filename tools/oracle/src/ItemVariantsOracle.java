import java.nio.file.*;
import java.security.MessageDigest;
import java.util.HexFormat;

/** Query item metadata behavior from the pinned client registry. */
public final class ItemVariantsOracle {
    public static void main(String[] args) throws Exception {
        Path jar = Path.of(args[0]).toRealPath();
        String hash = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
        if (!hash.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalArgumentException("wrong JAR");
        for (Class<?> c : new Class<?>[]{iz.class, gm.class, uu.class, ix.class})
            if (!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IllegalStateException("wrong class source");
        var out = System.out;
        System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
        Class.forName("uu");
        System.setOut(out);
        System.out.println("id\tmetadata\ticon\tsubtypes\tmax_damage\tmax_stack\tmerge_zero_count\tnew_slot_metadata_plus_one\tcoal_name");
        for (int id = 256; id <= 359; id++) {
            for (int meta = 0; meta < 16; meta++) {
                iz stack = new iz(id, 1, meta);
                ix inventory = new ix(null); inventory.a[0] = new iz(id, 1, 0);
                iz incoming = new iz(id, 1, meta);
                inventory.a(incoming);
                System.out.printf("%d\t%d\t%d\t%s\t%d\t%d\t%d\t%d\t%s%n", id, meta, stack.b(), stack.f(), stack.j(), stack.c(), inventory.a[0].a, inventory.a[1] == null ? 0 : inventory.a[1].i() + 1, id == 263 ? stack.l() : "-");
            }
        }
    }
}
