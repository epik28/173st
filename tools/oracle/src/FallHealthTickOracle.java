import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Native landing and living-tick counters, on an isolated remote empty world. */
public final class FallHealthTickOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    FallHealthOracle.Player p = (FallHealthOracle.Player) u.allocateInstance(FallHealthOracle.Player.class);
    FallHealthOracle.World world = (FallHealthOracle.World) u.allocateInstance(FallHealthOracle.World.class);
    p.aI = world; p.c = new ix(p); p.Y = 20; p.E = 20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl");
    p.advance(-5.0, false);
    p.advance(-0.25, true);
    System.setOut(out);
    // A remote world bypasses watcher updates; damage was applied before the
    // switch. Neither remote status nor overrides replace ls.U's counters.
    world.B = true;
    System.out.println("tick\thealth\ttimer\tanimation\tlast_damage\tprevious_health");
    for (int i = 0; i <= 21; i++) {
      System.out.printf("%d\t%d\t%d\t%d\t%d\t%d%n", i, p.Y, p.by, p.aa, p.lastDamage(), p.Z);
      if (i < 21) p.U();
    }
  }
}
