import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Captures ls.b(float) damage calls from the pinned Beta 1.7.3 client. */
public final class FallDamageOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class World extends fd {
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return 0; }
  }
  static final class Living extends ls {
    int damage;
    Living(fd w) { super(w); }
    @Override public boolean a(sn source, int value) { damage = value; return true; }
    void fall(float distance) { super.b(distance); }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    World world = (World) u.allocateInstance(World.class);
    int[] bits = {0x00000000, 0x3f800000, 0x403fffff, 0x40400000, 0x40400001,
      0x407fffff, 0x40800000, 0x40800001, 0x40a00000, 0x41240000, 0x43800000};
    System.out.println("distance_f32_hex\tdamage");
    for (int bit : bits) {
      Living living = (Living) u.allocateInstance(Living.class);
      living.aI = world;
      living.fall(Float.intBitsToFloat(bit));
      System.out.printf("%08x\t%d%n", bit, living.damage);
    }
  }
}
