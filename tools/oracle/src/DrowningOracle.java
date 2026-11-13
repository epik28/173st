import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Isolated native living-tick air counter and drowning damage events. */
public final class DrowningOracle {
  static final class World extends fd {
    int bubbles;
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return 0; }
    @Override public void a(sn e, byte event) { }
    @Override public void a(sn e, String sound, float volume, float pitch) { }
    @Override public void a(String name, double x, double y, double z, double vx, double vy, double vz) { if (name.equals("bubble")) bubbles++; }
  }
  static final class Player extends gs {
    boolean alive, submerged, breathes;
    int damage;
    Player() { super(null); }
    @Override public void v() { }
    @Override public void a(vr stat, int amount) { }
    @Override protected void ai() { }
    @Override public boolean k_() { return false; }
    @Override public boolean ah() { return false; }
    @Override public boolean W() { return alive; }
    @Override public boolean a(ln material) { return submerged && material == ln.g; }
    @Override public boolean d_() { return breathes; }
    @Override public boolean a(sn attacker, int amount) { damage += amount; return true; }
    void maxAir(int value) { bw = value; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    Player p = (Player) u.allocateInstance(Player.class); World w = (World) u.allocateInstance(World.class);
    p.aI = w; w.B = true; p.c = new ix(p); p.Y = 20; p.E = 20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    java.io.PrintStream out = System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl"); System.setOut(out);
    System.out.println("air_before\tmax_air\talive\tsubmerged\tbreathes\tair_after\tdamage\tbubbles");
    for (int air : new int[] {300, 1, 0, -1, -18, -19, -20, -21}) for (boolean alive : new boolean[] {false, true}) for (boolean submerged : new boolean[] {false, true}) for (boolean breathes : new boolean[] {false, true}) {
      p.bz = air; p.maxAir(300); p.alive = alive; p.submerged = submerged; p.breathes = breathes; p.damage = 0; w.bubbles = 0;
      System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); p.U(); System.setOut(out);
      System.out.printf("%d\t300\t%b\t%b\t%b\t%d\t%d\t%d%n", air, alive, submerged, breathes, p.bz, p.damage, w.bubbles);
    }
  }
}
