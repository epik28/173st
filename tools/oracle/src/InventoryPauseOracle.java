import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native screen pause predicate only; no renderer, window or save is opened. */
public final class InventoryPauseOracle {
  public static void main(String[] args) throws Exception {
    String hash = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(hash))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    ue inventory = (ue) u.allocateInstance(ue.class);
    System.out.println("screen\tpause\tmethod_owner");
    System.out.println("inventory\t" + inventory.c() + "\t" + ue.class.getMethod("c").getDeclaringClass().getName());
  }
}
