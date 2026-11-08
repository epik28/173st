import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Locale;
import java.util.Set;

/** Calls Beta 1.7.3's pinned obfuscated MathHelper class directly. */
public final class MathOracle {
  private static Method method(String name, Class<?>... parameters) throws Exception {
    return Class.forName("in").getDeclaredMethod(name, parameters);
  }

  private static int bits(String value) {
    if (!value.matches("[0-9a-fA-F]{8}")) throw new IllegalArgumentException("bad f32 bits");
    return (int) Long.parseLong(value, 16);
  }

  private static long dbits(String value) {
    if (!value.matches("[0-9a-fA-F]{16}")) throw new IllegalArgumentException("bad f64 bits");
    return Long.parseUnsignedLong(value, 16);
  }

  private static String eval(String op, String[] a) throws Exception {
    return switch (op) {
      case "sin", "cos" -> {
        if (a.length != 1) throw new IllegalArgumentException("arity");
        Method m = method(op.equals("sin") ? "a" : "b", float.class);
        float result = (float) m.invoke(null, Float.intBitsToFloat(bits(a[0])));
        yield String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(result));
      }
      case "sqrt_f32", "floor_f32", "abs_f32" -> {
        if (a.length != 1) throw new IllegalArgumentException("arity");
        String name = op.equals("sqrt_f32") ? "c" : op.equals("floor_f32") ? "d" : "e";
        Class<?> returnType = op.equals("floor_f32") ? int.class : float.class;
        Method m = method(name, float.class);
        Object result = m.invoke(null, Float.intBitsToFloat(bits(a[0])));
        yield returnType == int.class ? result.toString()
            : String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits((float) result));
      }
      case "sqrt_f64_to_f32", "floor_f64" -> {
        if (a.length != 1) throw new IllegalArgumentException("arity");
        Method m = method(op.equals("sqrt_f64_to_f32") ? "a" : "b", double.class);
        Object result = m.invoke(null, Double.longBitsToDouble(dbits(a[0])));
        yield op.equals("floor_f64") ? result.toString()
            : String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits((float) result));
      }
      case "abs_max_f64" -> {
        if (a.length != 2) throw new IllegalArgumentException("arity");
        double result = (double) method("a", double.class, double.class).invoke(null,
            Double.longBitsToDouble(dbits(a[0])), Double.longBitsToDouble(dbits(a[1])));
        yield String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(result));
      }
      case "floor_div_i32" -> {
        if (a.length != 2) throw new IllegalArgumentException("arity");
        int x = Integer.parseInt(a[0]), y = Integer.parseInt(a[1]);
        try { yield method("a", int.class, int.class).invoke(null, x, y).toString(); }
        catch (java.lang.reflect.InvocationTargetException e) {
          if (e.getCause() instanceof ArithmeticException) yield "error_arithmetic";
          throw e;
        }
      }
      case "null_or_empty" -> {
        if (a.length != 1) throw new IllegalArgumentException("arity");
        String value = a[0].equals("null") ? null : new String(hex(a[0]), StandardCharsets.UTF_8);
        yield (boolean) method("a", String.class).invoke(null, value) ? "1" : "0";
      }
      default -> throw new IllegalArgumentException("unknown operation");
    };
  }

  private static byte[] hex(String s) {
    if ((s.length() & 1) != 0 || !s.matches("[0-9a-fA-F]*")) throw new IllegalArgumentException("bad utf8 hex");
    byte[] out = new byte[s.length() / 2];
    for (int i = 0; i < out.length; i++) out[i] = (byte) Integer.parseInt(s.substring(2*i, 2*i+2), 16);
    return out;
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("usage: MathOracle CLIENT_JAR");
    if (!"af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c".equals(
        java.util.HexFormat.of().formatHex(java.security.MessageDigest.getInstance("SHA-256")
            .digest(java.nio.file.Files.readAllBytes(java.nio.file.Path.of(args[0])))))) {
      throw new IllegalArgumentException("unexpected client JAR SHA-256");
    }
    Set<String> ids = new HashSet<>();
    try (var in = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line; int lineNo = 0;
      while ((line = in.readLine()) != null) {
        lineNo++;
        try {
          String[] f = line.split("\\t", -1);
          if (f.length < 3 || f[0].isEmpty() || !f[0].matches("[a-zA-Z0-9_.:-]{1,96}")
              || !ids.add(f[0])) throw new IllegalArgumentException("bad or duplicate case id");
          String[] a = java.util.Arrays.copyOfRange(f, 2, f.length);
          System.out.print(f[0] + "\t" + eval(f[1], a) + "\n");
        } catch (Exception e) {
          System.err.println("MathOracle line " + lineNo + ": " + e.getClass().getSimpleName() + ": " + e.getMessage());
          System.exit(2);
        }
      }
    }
  }
}
