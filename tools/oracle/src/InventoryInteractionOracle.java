import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;

/** Captures native dw/aa click semantics without opening a window or world. */
public final class InventoryInteractionOracle {
    private static final class DummyPlayer extends gs {
        private DummyPlayer() { super(null); }
        public void v() { }
        public void a(iz stack) { }
    }
    private static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
    private static final sun.misc.Unsafe UNSAFE;
    static {
        try { Field f = sun.misc.Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); UNSAFE = (sun.misc.Unsafe) f.get(null); }
        catch (Exception e) { throw new ExceptionInInitializerError(e); }
    }
    private static String sha(byte[] b) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b)); }
    private static String stack(iz s) { return s == null ? "-" : s.c + ":" + s.a + ":" + s.i(); }
    private static String state(ix inv) {
        StringBuilder b = new StringBuilder("cursor=").append(stack(inv.i())).append("\tslots=");
        for (int i = 0; i < 36; i++) { if (i != 0) b.append(','); b.append(stack(inv.a[i])); }
        return b.toString();
    }
    private static void emit(String scenario, int step, String op, ix inv) { System.out.println(scenario + "\t" + step + "\t" + op + "\t" + state(inv)); }
    private static void put(ix inv, int slot, int id, int count, int damage) { inv.a[slot] = new iz(id, count, damage); }
    private static void scenario(String name, gs player, ix inv, aa c, int[] ops) {
        for (int i = 0; i < 36; i++) inv.a[i] = null;
        inv.b((iz) null);
        put(inv, 9, 1, 16, 0); put(inv, 10, 3, 4, 0); put(inv, 11, 332, 2, 0); put(inv, 12, 267, 1, 0);
        emit(name, 0, "initial", inv);
        int step = 1;
        for (int i = 0; i < ops.length; i += 3) { c.a(ops[i], ops[i + 1], ops[i + 2] != 0, player); emit(name, step++, "click:" + ops[i] + ":" + ops[i + 1] + ":shift=" + ops[i + 2], inv); }
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("usage: InventoryInteractionOracle <jar>");
        Path jar = Path.of(args[0]).toRealPath();
        if (!sha(Files.readAllBytes(jar)).equals(JAR_SHA)) throw new IllegalStateException("wrong pinned client JAR");
        java.io.PrintStream out = System.out;
        System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
        gs player = (gs) UNSAFE.allocateInstance(DummyPlayer.class); ix inv = new ix(null); player.c = inv; inv.d = player;
        Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(player, new Random(0));
        aa container = new aa(inv, true);
        System.out.println("jar_sha256\t" + JAR_SHA);
        scenario("left_pick_place", player, inv, container, new int[] {9,0,0, 13,0,0});
        scenario("right_split", player, inv, container, new int[] {9,1,0, 13,1,0});
        for (int i = 0; i < 36; i++) inv.a[i] = null; inv.b((iz) null); put(inv, 9, 1, 16, 0); put(inv, 10, 1, 4, 0); put(inv, 11, 332, 2, 0); put(inv, 12, 267, 1, 0); emit("merge_same", 0, "initial", inv); container.a(9,0,false,player); emit("merge_same", 1, "click:9:0:shift=0", inv); container.a(10,0,false,player); emit("merge_same", 2, "click:10:0:shift=0", inv);
        scenario("swap_different", player, inv, container, new int[] {9,0,0, 10,0,0});
        for (int i = 0; i < 36; i++) inv.a[i] = null; inv.b((iz) null); put(inv, 9, 1, 8, 0); emit("close_return", 0, "initial", inv); container.a(9,0,false,player); emit("close_return", 1, "click:9:0:shift=0", inv); container.a(player); emit("close_return", 2, "close", inv);
    }
}
