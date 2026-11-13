import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Captures the native gs.o peaceful-heal prefix, stopping before unrelated AI. */
public final class PeacefulRegenOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class Stop extends RuntimeException { }
  static final class Inventory extends ix {
    Inventory(gs owner) { super(owner); }
    @Override public void e() { throw new Stop(); }
  }
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
  }
  static final class World extends fd { World() { super((wt)null, "unused", 0L); } }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    Player p = (Player) u.allocateInstance(Player.class);
    World world = (World) u.allocateInstance(World.class); p.aI = world; p.c = new Inventory(p); p.E = 20;
    System.out.println("difficulty\ttick\thealth_before\ttimer_before\thealth_after\ttimer_after");
    for (int difficulty : new int[] {0, 1, 2, 3}) for (int tick : new int[] {0, 1, 19, 20, 21, 40}) for (int hp : new int[] {0, 1, 19, 20}) {
      world.q = difficulty; p.bt = tick; p.Y = hp; p.by = 3;
      try { p.o(); throw new AssertionError("inventory tick was not invoked"); } catch (Stop expected) { }
      System.out.printf("%d\t%d\t%d\t3\t%d\t%d%n", difficulty, tick, hp, p.Y, p.by);
    }
  }
}
