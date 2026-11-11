import java.nio.file.*;
import java.security.MessageDigest;
import java.util.HexFormat;

/** Capture native dye icon indices and inventory merge decisions. */
public final class DyeInventoryOracle {
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
        System.out.println("kind\tmetadata\tother\tvalue0\tvalue1");
        for (int damage = 0; damage < 16; damage++) {
            iz dye = new iz(351, 1, damage);
            System.out.printf("icon\t%d\t-\t%d\t%s%n", damage, dye.b(), dye.f());
            for (int other = 0; other < 16; other++) {
                ix inventory = new ix(null); inventory.a[0] = new iz(351, 10, damage);
                iz incoming = new iz(351, 5, other);
                if (!inventory.a(incoming)) throw new IllegalStateException("insert failed");
                System.out.printf("merge\t%d\t%d\t%d\t%d%n", damage, other, inventory.a[0].a,
                        inventory.a[1] == null ? 0 : inventory.a[1].i() + 1);
            }
        }
    }
}
