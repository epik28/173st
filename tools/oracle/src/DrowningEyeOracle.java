import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native gs.a(water) with player Y reconstructed from its foot-level box. */
public final class DrowningEyeOracle {
  static final class World extends fd {
    int id, metadata;
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return id; }
    @Override public int e(int x, int y, int z) { return metadata; }
  }
  static final class Player extends gs { Player() { super(null); } @Override public void v() { } @Override public void a(vr stat, int amount) { } }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    Player p = (Player) u.allocateInstance(Player.class); World w = (World) u.allocateInstance(World.class); p.aI = w;
    java.io.PrintStream out = System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
    System.out.println("foot_y\ty_size\tblock_id\tmetadata\teyes_in_water");
    for (double foot : new double[] {62.0, 62.1, 62.2, 62.26, 62.3, -0.1}) for (float ysize : new float[] {0.0f, 0.1f}) for (int id : new int[] {0, 8, 9}) for (int data = 0; data < 16; data++) {
      p.aM = -0.2; p.aN = foot + (double) 1.62f - (double) ysize; p.aO = 0.5; w.id = id; w.metadata = data;
      System.out.printf(java.util.Locale.ROOT, "%.9f\t%.9f\t%d\t%d\t%b%n", foot, ysize, id, data, p.a(ln.g));
    }
  }
}
