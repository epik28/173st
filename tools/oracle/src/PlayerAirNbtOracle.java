import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
/** Native NBT short getter used by sn.e(nu) for Air. */
public final class PlayerAirNbtOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    System.out.println("case\tvalue");
    nu n = new nu();
    System.out.println("missing\t" + n.d("Air"));
    for (short value : new short[]{Short.MIN_VALUE, -20, -1, 0, 1, 300, Short.MAX_VALUE}) {
      n.a("Air", value);
      System.out.println(value + "\t" + n.d("Air"));
    }
    n.a("Air", 300);
    try { n.d("Air"); System.out.println("wrong_type\taccepted"); }
    catch (ClassCastException e) { System.out.println("wrong_type\trejected"); }
  }
}
