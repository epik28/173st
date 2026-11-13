import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;

/** Captures native unarmored player health after sequential fall-damage calls. */
public final class FallHealthOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class World extends fd {
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return 0; }
    @Override public void a(sn e, byte event) { }
    @Override public void a(sn e, String sound, float volume, float pitch) { }
  }
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
    @Override public void a(vr stat, int amount) { }
    @Override protected void ai() { }
    @Override public boolean k_() { return false; }
    @Override public boolean ah() { return false; }
    @Override public boolean W() { return false; }
    void fall(float d) { super.b(d); }
    void advance(double delta, boolean onGround) { super.a(delta, onGround); }
    int lastDamage() { return au; }
  }
  static void emit(String name, Player p, float distance) {
    int before = p.Y, timerBefore = p.by, lastBefore = p.lastDamage(), previousBefore = p.Z;
    p.fall(distance);
    System.out.printf("%s\t%08x\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d%n", name,
        Float.floatToRawIntBits(distance), before, p.Y, before - p.Y, timerBefore,
        p.by, p.E, lastBefore, p.lastDamage(), previousBefore, p.Z, p.aa);
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    Player p = (Player) u.allocateInstance(Player.class);
    p.aI = (World) u.allocateInstance(World.class);
    p.c = new ix(p);
    p.Y = 20; p.E = 20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    java.io.PrintStream output = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl");
    System.setOut(output);
    System.out.println("case\tdistance_f32_hex\thealth_before\thealth_after\tapplied\ttimer_before\ttimer_after\tmax_timer\tlast_damage_before\tlast_damage_after\tprevious_health_before\tprevious_health_after\thurt_animation_after");
    emit("initial_hit", p, 5.0f);
    emit("same_damage_during_cooldown", p, 5.0f);
    emit("larger_damage_during_cooldown", p, 8.0f);
    p.by = 10;
    emit("half_cooldown_boundary", p, 5.0f);
    p.by = 0;
    emit("expired_cooldown", p, 5.0f);
  }
}
