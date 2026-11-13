import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Native player w_ order: living U, counter advance, peaceful heal prefix. */
public final class PeacefulPhaseOracle {
  static final class Stop extends RuntimeException { }
  static final class Inventory extends ix { Inventory(gs owner) { super(owner); } @Override public void e() { throw new Stop(); } }
  static final class World extends fd { World() { super((wt)null, "unused", 0L); } }
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
    @Override public boolean k_() { return false; }
    @Override public boolean ah() { return false; }
    @Override public boolean W() { return false; }
    void watcher() { bD = new ud(); bD.a(0, Byte.valueOf((byte)0)); }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(PeacefulRegenOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    Player p = (Player) u.allocateInstance(Player.class); World world = (World) u.allocateInstance(World.class);
    p.aI = world; p.c = new Inventory(p); p.Y = 18; p.E = 20; p.by = 3; p.bt = 18; p.watcher();
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    System.out.println("tick_before\ttick_after\thealth_before\thealth_after\ttimer_before\ttimer_after");
    for (int i = 0; i < 4; i++) {
      int tick = p.bt, health = p.Y, timer = p.by;
      try { p.w_(); throw new AssertionError("inventory tick was not invoked"); } catch (Stop expected) { }
      System.out.printf("%d\t%d\t%d\t%d\t%d\t%d%n", tick, p.bt, health, p.Y, timer, p.by);
    }
  }
}
