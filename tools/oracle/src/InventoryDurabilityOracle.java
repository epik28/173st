import java.nio.file.*;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.lang.reflect.Field;

/** Run original bb.b overlay logic with recording tessellator and no-op GL. */
public final class InventoryDurabilityOracle {
    public static void main(String[] args) throws Exception {
        Path jar = Path.of(args[0]).toRealPath();
        String hash = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
        if (!hash.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalArgumentException("wrong JAR");
        for (Class<?> c : new Class<?>[]{bb.class, iz.class, gm.class, uu.class})
            if (!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IllegalStateException("wrong class source");
        var out = System.out;
        System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
        Class.forName("uu");
        Field f = sun.misc.Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
        bb renderer = (bb)((sun.misc.Unsafe)f.get(null)).allocateInstance(bb.class);
        System.setOut(out);
        System.out.println("id\tdamage\tquads (x,y,width,height,rgb)");
        for (int id : new int[]{267, 268, 276, 359}) {
            int max = new iz(id, 1, 0).j();
            for (int damage : new int[]{0,1,max/4,max/2,max-1,max}) {
                System.out.printf("%d\t%d", id, damage);
                renderer.b(null, null, new iz(id, 1, damage), 10, 20);
                System.out.println();
            }
        }
    }
}
