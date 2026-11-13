import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Sequential real player fall hits through cooldown, real armor score and wear. */
public final class ArmorHealthOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    Field carry = gs.class.getDeclaredField("bO"); carry.setAccessible(true);
    FallHealthOracle.Player p = (FallHealthOracle.Player) u.allocateInstance(FallHealthOracle.Player.class);
    p.aI = (FallHealthOracle.World) u.allocateInstance(FallHealthOracle.World.class);
    p.c = new ix(p); p.Y = 20; p.E = 20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    java.io.PrintStream output = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl");
    p.c.b[0] = new iz(298, 1, 31);
    p.c.b[1] = new iz(307, 1, 80);
    System.setOut(output);
    System.out.println("case\tdistance\ttimer_before\thealth\ttimer_after\tlast_damage\tcarry\tscore\tslot0\tslot1");
    float[] distances = {4, 4, 7, 5, 6};
    int[] timers = {0, 20, 20, 10, 0};
    for (int i = 0; i < distances.length; i++) {
      p.by = timers[i]; p.fall(distances[i]);
      System.out.printf("hit_%d\t%.0f\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%s%n", i, distances[i], timers[i], p.Y, p.by, p.lastDamage(), carry.getInt(p), p.c.f(), item(p.c.b[0]), item(p.c.b[1]));
    }
  }
  static String item(iz stack) { return stack == null ? "-" : Integer.toString(stack.h()); }
}
