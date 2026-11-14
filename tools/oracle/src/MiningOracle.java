import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Block hardness and the native inventory's speed/harvest dispatch. */
public final class MiningOracle {
  static final class Player extends gs {
    boolean submerged;
    Player() { super(null); }
    @Override public void v() { }
    @Override public boolean a(ln material) { return submerged && material == ln.g; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Player player = (Player) ((Unsafe) f.get(null)).allocateInstance(Player.class);
    player.c = new ix(player);
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl");
    System.setOut(out);
    Field hardness = uu.class.getDeclaredField("bo"); hardness.setAccessible(true);
    System.out.println("block\titem\thardness_bits\tspeed_bits\tharvest\tground\tair\twater\twater_air");
    for (int block = 1; block < 256; block++) if (uu.m[block] != null) {
      for (int item = 0; item <= 359; item++) if (item == 0 || (item >= 256 && gm.c[item] != null)) {
        player.c.a[0] = item == 0 ? null : new iz(item, 1, 0);
        uu target = uu.m[block];
        System.out.printf("%d\t%d\t%08x\t%08x\t%d", block, item,
            Float.floatToRawIntBits(hardness.getFloat(target)), Float.floatToRawIntBits(player.c.a(target)), player.c.b(target) ? 1 : 0);
        for (int mode = 0; mode < 4; mode++) {
          player.aX = (mode & 1) == 0; player.submerged = (mode & 2) != 0;
          System.out.printf("\t%08x", Float.floatToRawIntBits(target.a(player)));
        }
        System.out.println();
      }
    }
  }
}
