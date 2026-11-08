import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.nio.ByteBuffer;
import java.nio.charset.CodingErrorAction;
import java.nio.charset.StandardCharsets;

/** Direct JVM primitive casts and String.hashCode reference adapter. */
public final class JavaCompatOracle {
  private static String utf8(String hex) throws Exception {
    if ((hex.length() & 1) != 0 || !hex.matches("[0-9a-f]*")) {
      throw new IllegalArgumentException("invalid UTF-8 hex");
    }
    byte[] bytes = new byte[hex.length() / 2];
    for (int i = 0; i < bytes.length; i++) {
      bytes[i] = (byte) Integer.parseInt(hex.substring(i * 2, i * 2 + 2), 16);
    }
    return StandardCharsets.UTF_8.newDecoder()
        .onMalformedInput(CodingErrorAction.REPORT)
        .onUnmappableCharacter(CodingErrorAction.REPORT)
        .decode(ByteBuffer.wrap(bytes)).toString();
  }

  private static String eval(String op, String arg) throws Exception {
    return switch (op) {
      case "f32_to_i32" -> Integer.toString((int) Float.intBitsToFloat((int) bits(arg, 8)));
      case "f64_to_i32" -> Integer.toString((int) Double.longBitsToDouble(bits(arg, 16)));
      case "f32_to_i64" -> Long.toString((long) Float.intBitsToFloat((int) bits(arg, 8)));
      case "f64_to_i64" -> Long.toString((long) Double.longBitsToDouble(bits(arg, 16)));
      case "string_hash_code" -> Integer.toString(utf8(arg).hashCode());
      default -> throw new IllegalArgumentException("unknown operation");
    };
  }

  private static long bits(String value, int width) {
    if (value.length() != width || !value.matches("[0-9a-f]+")) {
      throw new IllegalArgumentException("bad raw bits");
    }
    return Long.parseUnsignedLong(value, 16);
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 0) throw new IllegalArgumentException("unexpected arguments");
    var ids = new java.util.HashSet<String>();
    try (var reader = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line;
      int lineNo = 0;
      while ((line = reader.readLine()) != null) {
        lineNo++;
        try {
          String[] fields = line.split("\\t", -1);
          if (fields.length != 3 || !fields[0].matches("[A-Za-z0-9_.:-]{1,96}")
              || !ids.add(fields[0])) throw new IllegalArgumentException("bad or duplicate ID");
          System.out.print(fields[0] + "\t" + eval(fields[1], fields[2]) + "\n");
        } catch (Exception error) {
          System.err.println("JavaCompatOracle line " + lineNo + ": " + error);
          System.exit(2);
        }
      }
    }
  }
}
