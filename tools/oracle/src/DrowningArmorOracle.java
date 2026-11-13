import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Native ls.U drowning tick using a real player damage and armor implementation. */
public final class DrowningArmorOracle {
  static final class World extends fd {
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return 0; }
    @Override public void a(sn e, byte event) { }
    @Override public void a(sn e, String sound, float volume, float pitch) { }
    @Override public void a(String s, double x, double y, double z, double vx, double vy, double vz) { }
  }
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
    @Override public void a(vr stat, int amount) { }
    @Override protected void ai() { }
    @Override public boolean k_() { return false; }
    @Override public boolean ah() { return false; }
    @Override public boolean W() { return true; }
    @Override public boolean a(ln material) { return true; }
    @Override public boolean d_() { return false; }
    void initializeWatcher() { bD = new ud(); bD.a(0, Byte.valueOf((byte)0)); }
    int lastDamage() { return au; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    Field carry = gs.class.getDeclaredField("bO"); carry.setAccessible(true);
    Player p = (Player) u.allocateInstance(Player.class);
    p.aI = (World) u.allocateInstance(World.class); p.c = new ix(p); p.Y = 20; p.E = 20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    p.initializeWatcher();
    java.io.PrintStream output = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl");
    p.c.b[0] = new iz(298, 1, 31); p.c.b[1] = new iz(307, 1, 80);
    System.setOut(output);
    System.out.println("case\ttimer_before\thealth\ttimer_after\tlast_damage\tcarry\tscore\tslot0\tslot1\tair");
    for (int timer : new int[] {0, 20, 10, 0}) {
      p.by = timer; p.bz = -19; p.U();
      System.out.printf("drown\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%s\t%d%n", timer, p.Y, p.by, p.lastDamage(), carry.getInt(p), p.c.f(), item(p.c.b[0]), item(p.c.b[1]), p.bz);
    }
  }
  static String item(iz stack) { return stack == null ? "-" : Integer.toString(stack.h()); }
}
