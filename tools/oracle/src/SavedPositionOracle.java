import java.nio.file.*;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.lang.reflect.Field;

/** Native player NBT roundtrip with synthetic constructor-free world. */
public final class SavedPositionOracle {
    static final class World extends fd {
        World() { super((wt)null, "unused", 0L); throw new AssertionError(); }
        @Override public br u() { return new br(0, 64, 0); }
    }
    static final class Player extends gs {
        Player(fd world) { super(world); }
        @Override public void v() { }
    }
    static String bits(double x) { return String.format("%016x", Double.doubleToRawLongBits(x)); }
    public static void main(String[] args) throws Exception {
        Path jar = Path.of(args[0]).toRealPath();
        String hash = HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
        if (!hash.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalArgumentException("wrong JAR");
        for (Class<?> c : new Class<?>[]{sn.class, gs.class, fd.class, nu.class})
            if (!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IllegalStateException("wrong class source");
        java.io.PrintStream out = System.out;
        System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
        Class.forName("uu");
        Field field = sun.misc.Unsafe.class.getDeclaredField("theUnsafe"); field.setAccessible(true);
        World world = (World)((sun.misc.Unsafe)field.get(null)).allocateInstance(World.class);
        Player first = new Player(world);
        first.e(-0.1, 65.0, 16.1);
        nu nbt = new nu(); first.d(nbt);
        sp pos = nbt.l("Pos");
        Player loaded = new Player(world); loaded.e(nbt);
        System.setOut(out);
        System.out.println("pos_x\tpos_y\tpos_z\tloaded_min_y\tloaded_max_y\ty_offset");
        System.out.println(bits(((sz)pos.a(0)).a)+"\t"+bits(((sz)pos.a(1)).a)+"\t"+bits(((sz)pos.a(2)).a)+"\t"+bits(loaded.aW.b)+"\t"+bits(loaded.aW.e)+"\t"+bits(loaded.bf));
    }
}
