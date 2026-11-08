import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.lang.reflect.Field;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.ArrayList;
import java.util.Base64;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;

/** Captures submitted standard-cube face vertices from the pinned native renderer. */
public final class UvOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static int invocations;

  static String sha(Path path) throws Exception {
    return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(java.nio.file.Files.readAllBytes(path)));
  }

  static void requireJar(Class<?> type, Path jar) throws Exception {
    Path source = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
    if (!source.equals(jar)) throw new IllegalStateException("class not loaded from pinned JAR: " + type.getName());
  }

  static String classSha(Class<?> type) throws Exception {
    try (InputStream in = type.getResourceAsStream("/" + type.getName() + ".class")) {
      if (in == null) throw new IllegalStateException("missing class bytes: " + type.getName());
      return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(in.readAllBytes()));
    }
  }

  static String sourceSha(Class<?> type) throws Exception {
    Path source = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
    return sha(source);
  }

  record FaceCase(String id, int face, int tile, int x, int y, int z) {}

  static Object defaultValue(Class<?> type) {
    if (type == boolean.class) return false;
    if (type == int.class) return 0;
    if (type == float.class) return 1.0f;
    if (type == double.class) return 0.0d;
    return null;
  }

  static String bits(int value) { return String.format(Locale.ROOT, "%08x", value); }

  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("UvOracle CLIENT_JAR");
    Path jar = Path.of(args[0]).toRealPath();
    if (!sha(jar).equals(JAR_SHA)) throw new IllegalStateException("wrong client JAR SHA-256");
    requireJar(cv.class, jar); requireJar(nw.class, jar); requireJar(uu.class, jar);

    // Capture the registry's recipe-count output as provenance instead of mixing it with TSV.
    var report = System.out;
    var bootstrap = new java.io.ByteArrayOutputStream();
    System.setOut(new java.io.PrintStream(bootstrap, true, StandardCharsets.UTF_8));
    uu block = uu.m[1];
    if (block == null) throw new IllegalStateException("native stone block missing");
    System.setOut(report);
    byte[] bootstrapBytes = bootstrap.toByteArray();
    Class<?> lwjglClass = org.lwjgl.opengl.GLContext.class;
    String lwjglSha = sourceSha(lwjglClass);
    xp world = (xp)Proxy.newProxyInstance(xp.class.getClassLoader(), new Class<?>[]{xp.class},
      (proxy, method, values) -> defaultValue(method.getReturnType()));
    cv renderer = new cv(world);
    nw tessellator = nw.a;
    Field verticesField = nw.class.getDeclaredField("g"); verticesField.setAccessible(true);
    Field offsetField = nw.class.getDeclaredField("o"); offsetField.setAccessible(true);
    int[] vertices = (int[])verticesField.get(tessellator);
    List<FaceCase> cases = new ArrayList<>(); Set<String> ids = new HashSet<>();
    try (var reader = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
      String line; int lineNumber = 0;
      while ((line = reader.readLine()) != null) {
        lineNumber++;
        if (line.isEmpty() || line.startsWith("#")) continue;
        String[] fields = line.split("\\t", -1);
        if (fields.length != 6) throw new IllegalArgumentException("line " + lineNumber + ": expected id,face,tile,x,y,z");
        String id = fields[0];
        if (!id.matches("[A-Za-z0-9._-]{1,64}") || !ids.add(id)) throw new IllegalArgumentException("line " + lineNumber + ": invalid or duplicate id");
        int face = Integer.parseInt(fields[1]), tile = Integer.parseInt(fields[2]);
        int x = Integer.parseInt(fields[3]), y = Integer.parseInt(fields[4]), z = Integer.parseInt(fields[5]);
        if (face < 0 || face > 5 || tile < 0 || tile > 255 || x < -64 || x > 64 || y < -64 || y > 64 || z < -64 || z > 64)
          throw new IllegalArgumentException("line " + lineNumber + ": input out of range");
        if (cases.size() == 4096) throw new IllegalArgumentException("case limit exceeded");
        cases.add(new FaceCase(id, face, tile, x, y, z));
      }
    }
    tessellator.b(); // begin native format 7; no draw/flush and no GL context required.
    report.println("#UV_NATIVE\t1\tjar=" + JAR_SHA + "\tcv=" + classSha(cv.class) + "\tnw=" + classSha(nw.class) +
      "\tuu=" + classSha(uu.class) + "\tlwjgl=" + lwjglSha + "\tapi=cv.a-f(uu,DDD,I)->nw.a(DDDDD)\tpacked=float32-bits");
    report.println("#bootstrap\tbytes=" + bootstrapBytes.length + "\tsha256=" + HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bootstrapBytes)));
    report.println("#bootstrap_data\tbase64=" + Base64.getEncoder().encodeToString(bootstrapBytes));
    for (FaceCase item : cases) {
        String id = item.id(); int face = item.face(), tile = item.tile(), x = item.x(), y = item.y(), z = item.z();
        int before = (int)offsetField.get(tessellator);
        switch (face) {
          case 0 -> renderer.a(block, (double)x, (double)y, (double)z, tile);
          case 1 -> renderer.b(block, (double)x, (double)y, (double)z, tile);
          case 2 -> renderer.c(block, (double)x, (double)y, (double)z, tile);
          case 3 -> renderer.d(block, (double)x, (double)y, (double)z, tile);
          case 4 -> renderer.e(block, (double)x, (double)y, (double)z, tile);
          case 5 -> renderer.f(block, (double)x, (double)y, (double)z, tile);
          default -> throw new AssertionError();
        }
        int after = (int)offsetField.get(tessellator);
        if (after - before != 48) throw new IllegalStateException("native face " + face + " submitted " + ((after - before) / 8) + " vertices (offset " + before + " -> " + after + ")");
        for (int corner = 0; corner < 6; corner++) {
          int at = before + corner * 8;
          report.println(id + "\t" + corner + "\t" + bits(vertices[at]) + "\t" + bits(vertices[at + 1]) + "\t" +
            bits(vertices[at + 2]) + "\t" + bits(vertices[at + 3]) + "\t" + bits(vertices[at + 4]));
        }
        invocations++;
    }
    report.println("#calls\tface=" + invocations + "\tvertices=" + (invocations * 6) + "\tglFlush=0");
    report.flush();
  }
}
