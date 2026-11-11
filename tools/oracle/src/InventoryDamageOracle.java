import java.nio.file.*;
import java.security.MessageDigest;
import java.util.HexFormat;

/** Native registry durability and icon ranges, queried without a world. */
public final class InventoryDamageOracle {
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
        System.out.println("id\tmax_damage\ticon\ticons_constant");
        for (int id = 256; id <= 359; id++) {
            iz stack = new iz(id, 1, 0);
            int max = stack.j(), icon = stack.b(); boolean constant = true;
            for (int damage = 0; damage <= max; damage++) if (new iz(id, 1, damage).b() != icon) constant = false;
            System.out.printf("%d\t%d\t%d\t%s%n", id, max, icon, constant);
        }
    }
}
