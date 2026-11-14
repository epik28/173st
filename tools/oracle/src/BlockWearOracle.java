import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Capture held-item damage after native ItemStack.onBlockDestroyed. */
public final class BlockWearOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    FoodOracle.Player player = (FoodOracle.Player) ((Unsafe) f.get(null)).allocateInstance(FoodOracle.Player.class);
    player.c = new ix(player);
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl");
    System.setOut(out);
    System.out.println("id\tblock\tmax_damage\tdamage_after\tcount_after\tboundary_damage\tboundary_count");
    for (int id = 256; id <= 359; id++) if (gm.c[id] != null) {
      for (int block = 1; block < 256; block++) if (uu.m[block] != null) {
        iz stack = new iz(id, 1, 0);
        stack.a(block, 0, 0, 0, player);
        iz boundary = new iz(id, 1, stack.j());
        boundary.a(block, 0, 0, 0, player);
        System.out.printf("%d\t%d\t%d\t%d\t%d\t%d\t%d%n", id, block, stack.j(), stack.h(), stack.a, boundary.h(), boundary.a);
      }
    }
  }
}
