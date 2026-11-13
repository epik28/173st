import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native armor registry, durability-dependent score and boundary wear. */
public final class ArmorEquipmentOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
    @Override public void a(vr stat, int amount) { }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("gm");
    System.setOut(out);
    Player p = (Player) u.allocateInstance(Player.class);
    System.out.println("id\tprotection\tmax_damage\tscore_fresh\tscore_half\tscore_at_max\tworn_at_max\tbroken_next");
    for (int id = 0; id < gm.c.length; id++) {
      if (!(gm.c[id] instanceof wa armor)) continue;
      ix inventory = new ix(p);
      iz stack = new iz(id, 1, 0);
      int max = stack.j();
      inventory.b[0] = stack;
      int fresh = inventory.f();
      inventory.b[0] = new iz(id, 1, max / 2);
      int half = inventory.f();
      inventory.b[0] = new iz(id, 1, max - 1);
      inventory.e(1);
      int atMax = inventory.f();
      int worn = inventory.b[0].h();
      inventory.e(1);
      System.out.printf("%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d%n", id, armor.bl, max, fresh, half, atMax, worn, inventory.b[0] == null ? 1 : 0);
    }
  }
}
