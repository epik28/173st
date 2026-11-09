import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;

/** Reads the original Beta 1.7.3 block material arrays from the pinned client JAR. */
public final class LightMaterialOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final byte[] REQUEST = "registry\tall\n".getBytes(StandardCharsets.US_ASCII);

  static String sha(byte[] bytes) throws Exception {
    return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
  }

  public static void main(String[] args) {
    try {
      if (args.length != 1) throw new IOException("usage: LightMaterialOracle CLIENT_JAR");
      Path jar = Path.of(args[0]).toRealPath();
      if (!sha(Files.readAllBytes(jar)).equals(JAR_SHA))
        throw new IOException("wrong client JAR SHA-256");
      byte[] request = System.in.readNBytes(65);
      if (!java.util.Arrays.equals(request, REQUEST) || System.in.read() != -1)
        throw new IOException("request must be exactly 'registry\\tall\\n'");

      Path source = Path.of(uu.class.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
      if (!source.equals(jar)) throw new IOException("uu class not loaded from configured client JAR: " + source);
      java.io.PrintStream stdout = System.out;
      java.io.ByteArrayOutputStream bootstrap = new java.io.ByteArrayOutputStream();
      System.setOut(new java.io.PrintStream(bootstrap, true, StandardCharsets.UTF_8));
      uu[] blocks;
      int[] opacityTable, emissionTable;
      try {
        blocks = uu.m;
        opacityTable = uu.q;
        emissionTable = uu.s;
      } finally {
        System.setOut(stdout);
      }
      if (!sha(bootstrap.toByteArray()).equals("0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e"))
        throw new IOException("unexpected original block-registry bootstrap bytes: sha=" + sha(bootstrap.toByteArray()));
      if (blocks.length != 256 || opacityTable.length != 256 || emissionTable.length != 256)
        throw new IOException("native uu registry arrays must each contain 256 entries");

      StringBuilder out = new StringBuilder(8192);
      for (int id = 0; id < 256; id++) {
        int opacity = opacityTable[id], emission = emissionTable[id];
        if (opacity < 0 || opacity > 255) throw new IOException("opacity out of unsigned-byte range at ID " + id);
        if (emission < 0 || emission > 15) throw new IOException("emission out of nibble range at ID " + id);
        out.append(id).append('\t').append(blocks[id] == null ? 0 : 1)
            .append('\t').append(opacity).append('\t').append(emission).append('\n');
      }
      System.out.write(out.toString().getBytes(StandardCharsets.US_ASCII));
      System.out.flush();
    } catch (Throwable failure) {
      System.err.println("LightMaterialOracle: " + failure.getMessage());
      System.exit(1);
    }
  }
}
