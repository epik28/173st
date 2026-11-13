import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;
/** Executes native living entity NBT read for health and animation counters. */
public final class PlayerHealthNbtOracle {
  static final class Living extends ls { Living() { super(null); } }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    System.out.println("case\thealth\thurt_time\tdeath_time\tattack_time");
    for (int health : new int[]{-1, 0, 1, 10, 20, 21}) {
      Living p = (Living) u.allocateInstance(Living.class); nu n = new nu();
      n.a("Health", (short) health); n.a("HurtTime", (short) 5); n.a("DeathTime", (short) 6); n.a("AttackTime", (short) 7);
      p.a(n); System.out.printf("%d\t%d\t%d\t%d\t%d%n", health, p.Y, p.aa, p.ad, p.ae);
    }
    Living p = (Living) u.allocateInstance(Living.class); p.a(new nu());
    System.out.printf("missing\t%d\t%d\t%d\t%d%n", p.Y, p.aa, p.ad, p.ae);
  }
}
