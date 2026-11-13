import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Combined native armor score before and after shared wear. */
public final class ArmorEquipmentMixOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(ArmorEquipmentOracle.SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    ArmorEquipmentOracle.Player p = (ArmorEquipmentOracle.Player) ((Unsafe) f.get(null)).allocateInstance(ArmorEquipmentOracle.Player.class);
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("gm");
    System.setOut(out);
    ix inv = new ix(p);
    inv.b[0] = new iz(298, 1, 10);
    inv.b[1] = new iz(307, 1, 80);
    inv.b[2] = new iz(313, 1, 200);
    System.out.println("wear\tscore\tslot0_damage\tslot1_damage\tslot2_damage");
    for (int wear : new int[] {0, 2, 21}) {
      inv.e(wear);
      System.out.printf("%d\t%d\t%s\t%s\t%s%n", wear, inv.f(), value(inv.b[0]), value(inv.b[1]), value(inv.b[2]));
    }
  }
  static String value(iz item) { return item == null ? "-" : Integer.toString(item.h()); }
}
