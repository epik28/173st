import java.io.*;
import java.security.MessageDigest;
import java.util.*;

/** Adapter around the original Beta 1.7.3 NBT classes (ij/nu). */
public final class NbtOracle {
  private static byte[] hex(String s) {
    if (s.isEmpty() || (s.length() & 1) != 0 || !s.matches("[0-9a-fA-F]+")) throw new IllegalArgumentException("invalid hex");
    byte[] out = new byte[s.length() / 2];
    for (int i = 0; i < out.length; i++) out[i] = (byte) Integer.parseInt(s.substring(i * 2, i * 2 + 2), 16);
    return out;
  }
  private static String hex(byte[] b) {
    StringBuilder s = new StringBuilder(b.length * 2);
    for (byte x : b) s.append(String.format(Locale.ROOT, "%02x", x & 255));
    return s.toString();
  }
  private static byte[] roundTrip(byte[] input) throws IOException {
    DataInputStream in = new DataInputStream(new ByteArrayInputStream(input));
    ij root = ij.b(in);
    if (root.a() == 0) throw new IOException("Root tag cannot be End (wrapper guard)");
    if (in.available() != 0) throw new IOException("trailing bytes: " + in.available());
    ByteArrayOutputStream bytes = new ByteArrayOutputStream();
    ij.a(root, new DataOutputStream(bytes));
    return bytes.toByteArray();
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("usage: NbtOracle JAR");
    File jar = new File(args[0]);
    MessageDigest md = MessageDigest.getInstance("SHA-256");
    try (InputStream in = new FileInputStream(jar)) { byte[] b = new byte[8192]; int n; while ((n = in.read(b)) >= 0) if (n > 0) md.update(b, 0, n); }
    System.err.println("jar_sha256=" + hex(md.digest()));
    BufferedReader reader = new BufferedReader(new InputStreamReader(System.in, "UTF-8"));
    String line; int count = 0; long total = 0;
    Set<String> ids = new HashSet<>();
    while ((line = reader.readLine()) != null) {
      if (line.isEmpty() || line.startsWith("#")) continue;
      String[] f = line.split("\\t", -1);
      if (f.length != 2 || !f[0].matches("[A-Za-z0-9_-]+")) throw new IllegalArgumentException("expected id<TAB>hex");
      if (!ids.add(f[0])) throw new IllegalArgumentException("duplicate id");
      if (++count > 100000) throw new IllegalArgumentException("case limit");
      total += f[1].length() / 2;
      if (total > 64L * 1024 * 1024) throw new IllegalArgumentException("input byte limit");
      byte[] input = hex(f[1]);
      byte[] output = roundTrip(input);
      System.out.println(f[0] + "\t0\t" + hex(output) + "\t" + input.length);
    }
    System.err.println("cases=" + count + " classes=ij,nu");
  }
}
