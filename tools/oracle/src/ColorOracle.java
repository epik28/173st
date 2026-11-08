import java.awt.image.BufferedImage;
import java.io.ByteArrayInputStream;
import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.InvocationHandler;
import java.lang.reflect.Method;
import java.lang.reflect.Proxy;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.jar.JarFile;
import javax.imageio.ImageIO;

/** Native climate/palette oracle. All game calls execute from the pinned client JAR. */
public final class ColorOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final String GRASS_SHA = "d9b2d9073f06013b9f36b1c61a91678be478760abff31fae942ee52ad898ca1b";
  static final String FOLIAGE_SHA = "644a527dd7daea5a195bbed056a6a41e295b6043a05621f4c5c263a25fb171cc";
  static final int MAX_INPUT = 4 * 1024 * 1024;
  static final int MAX_ROWS = 4096;
  static int seedReads, directClimateCalls, grassCalls, foliageCalls;
  static int directGrassPaletteCalls, directFoliagePaletteCalls;

  static final class SeedOnlyWorld extends fd {
    long selectedSeed;
    SeedOnlyWorld() { super((wt)null, "", 0L); }
    @Override public long s() { seedReads++; return selectedSeed; }
  }

  record Input(String id, String type, long seed, int x, int z, double temperature, double rainfall) {}

  static final class NativeWorld implements InvocationHandler {
    final xv manager;
    int metadata;
    final xp proxy;
    NativeWorld(xv manager) {
      this.manager = manager;
      this.proxy = (xp)Proxy.newProxyInstance(xp.class.getClassLoader(), new Class<?>[]{xp.class}, this);
    }
    @Override public Object invoke(Object ignored, Method method, Object[] args) {
      if (method.getName().equals("a") && method.getParameterCount() == 0) return manager;
      if (method.getName().equals("e") && method.getParameterCount() == 3) return metadata;
      Class<?> result = method.getReturnType();
      if (result == boolean.class) return false;
      if (result == int.class) return 0;
      if (result == float.class) return 1.0f;
      if (result == double.class) return 0.0d;
      return null;
    }
  }

  static String sha(byte[] value) throws Exception {
    return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(value));
  }
  static String sha(Path path) throws Exception { return sha(Files.readAllBytes(path)); }
  static String classSha(Class<?> type) throws Exception {
    try (var input = type.getResourceAsStream("/" + type.getName() + ".class")) {
      if (input == null) throw new IOException("missing native class bytes: " + type.getName());
      return sha(input.readAllBytes());
    }
  }
  static Path sourcePath(Class<?> type) throws Exception {
    return Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
  }
  static void requireJar(Class<?> type, Path jar) throws Exception {
    if (!sourcePath(type).equals(jar)) throw new IOException("class not loaded from pinned client JAR: " + type.getName());
  }
  static String hex32(int value) { return String.format(Locale.ROOT, "%08x", value); }
  static String hex64(long value) { return String.format(Locale.ROOT, "%016x", value); }

  static int[] palette(Path path, String expectedHash) throws Exception {
    byte[] bytes = Files.readAllBytes(path);
    if (!sha(bytes).equals(expectedHash)) throw new IOException("wrong palette SHA-256: " + path.getFileName());
    BufferedImage image = ImageIO.read(new ByteArrayInputStream(bytes));
    if (image == null || image.getWidth() != 256 || image.getHeight() != 256)
      throw new IOException("palette must decode as 256x256 PNG: " + path.getFileName());
    return image.getRGB(0, 0, 256, 256, null, 0, 256);
  }

  static List<Input> parse(byte[] bytes) throws Exception {
    if (bytes.length == 0 || bytes.length > MAX_INPUT || bytes[bytes.length - 1] != '\n')
      throw new IOException("input must be nonempty, <=4MiB and LF terminated");
    for (byte value : bytes) if ((value & 0x80) != 0 || value == '\r') throw new IOException("input must be ASCII with LF line endings");
    String text = new String(bytes, StandardCharsets.US_ASCII);
    String[] lines = text.substring(0, text.length() - 1).split("\\n", -1);
    if (lines.length > MAX_ROWS) throw new IOException("row limit exceeded");
    List<Input> cases = new ArrayList<>(lines.length); Set<String> ids = new HashSet<>();
    for (int index = 0; index < lines.length; index++) {
      String line = lines[index]; String[] columns = line.split("\\t", -1);
      if (line.isEmpty() || columns.length < 2 || !columns[0].matches("[A-Za-z0-9_.:-]{1,96}") || !ids.add(columns[0]))
        throw new IOException("line " + (index + 1) + ": invalid/duplicate id or empty row");
      if (columns[1].equals("climate")) {
        if (columns.length != 5) throw new IOException("line " + (index + 1) + ": climate requires 5 columns");
        long seed = signed(columns[2], Long.MIN_VALUE, Long.MAX_VALUE);
        int x = (int)signed(columns[3], -30_000_000, 30_000_000);
        int z = (int)signed(columns[4], -30_000_000, 30_000_000);
        cases.add(new Input(columns[0], "climate", seed, x, z, 0, 0));
      } else if (columns[1].equals("palette")) {
        if (columns.length != 4) throw new IOException("line " + (index + 1) + ": palette requires 4 columns");
        double temperature = rawFinite(columns[2]), rainfall = rawFinite(columns[3]);
        if (temperature < 0 || temperature > 1 || rainfall < 0 || rainfall > 1)
          throw new IOException("line " + (index + 1) + ": palette climate outside 0..1");
        cases.add(new Input(columns[0], "palette", 0, 0, 0, temperature, rainfall));
      } else throw new IOException("line " + (index + 1) + ": unknown row type");
    }
    return cases;
  }

  static long signed(String value, long min, long max) throws IOException {
    if (!value.matches("0|-?[1-9][0-9]*")) throw new IOException("noncanonical signed decimal: " + value);
    try {
      long result = Long.parseLong(value);
      if (result < min || result > max) throw new IOException("integer outside allowed range: " + value);
      return result;
    } catch (NumberFormatException failure) { throw new IOException("integer outside allowed range: " + value, failure); }
  }
  static double rawFinite(String value) throws IOException {
    if (!value.matches("[0-9a-f]{16}")) throw new IOException("expected lowercase raw f64 hex: " + value);
    double result = Double.longBitsToDouble(Long.parseUnsignedLong(value, 16));
    if (!Double.isFinite(result)) throw new IOException("nonfinite palette climate");
    return result;
  }

  static SeedOnlyWorld synthetic(long seed) throws Exception {
    Class<?> unsafeClass = Class.forName("sun.misc.Unsafe");
    Field field = unsafeClass.getDeclaredField("theUnsafe"); field.setAccessible(true);
    Object unsafe = field.get(null);
    Object allocated = unsafeClass.getMethod("allocateInstance", Class.class).invoke(unsafe, SeedOnlyWorld.class);
    SeedOnlyWorld world = (SeedOnlyWorld)allocated;
    world.selectedSeed = seed;
    return world;
  }

  static xv manager(long seed, Map<Long, xv> cache) throws Exception {
    xv found = cache.get(seed);
    if (found != null) return found;
    xv created = new xv(synthetic(seed)); cache.put(seed, created); return created;
  }

  static void verifyMethods() throws Exception {
    if (xv.class.getMethod("a", int.class, int.class, int.class, int.class).getDeclaringClass() != xv.class ||
        wp.class.getMethod("b", xp.class, int.class, int.class, int.class).getDeclaringClass() != wp.class ||
        bk.class.getMethod("b", xp.class, int.class, int.class, int.class).getDeclaringClass() != bk.class ||
        ia.class.getMethod("a", double.class, double.class).getDeclaringClass() != ia.class ||
        jh.class.getMethod("a", double.class, double.class).getDeclaringClass() != jh.class)
      throw new IOException("unexpected native declaring class/signature");
  }

  public static void main(String[] args) {
    try {
      if (args.length != 3) throw new IllegalArgumentException("ColorOracle CLIENT_JAR GRASS_PNG FOLIAGE_PNG");
      Path jar = Path.of(args[0]).toRealPath();
      if (!sha(jar).equals(JAR_SHA)) throw new IOException("wrong client JAR SHA-256");
      List<Input> cases = parse(System.in.readNBytes(MAX_INPUT + 1));
      int[] grassPixels = palette(Path.of(args[1]).toRealPath(), GRASS_SHA);
      int[] foliagePixels = palette(Path.of(args[2]).toRealPath(), FOLIAGE_SHA);
      verifyMethods();
      List<Class<?>> audited = List.of(fd.class, xv.class, ug.class, df.class, cc.class, kd.class,
        wp.class, bk.class, uu.class, ia.class, jh.class);
      for (Class<?> type : audited) requireJar(type, jar);
      String nativeCodeSource = sourcePath(xv.class).toUri().toASCIIString();
      for (Class<?> type : audited) if (!sourcePath(type).toUri().toASCIIString().equals(nativeCodeSource))
        throw new IOException("native classes do not share one code source");

      // Suppress and retain only the original block registry's bootstrap printout.
      var stdout = System.out;
      var bootstrap = new java.io.ByteArrayOutputStream();
      System.setOut(new java.io.PrintStream(bootstrap, true, StandardCharsets.UTF_8));
      wp grassBlock = uu.v;
      bk leafBlock = (bk)uu.m[18];
      if (grassBlock == null || leafBlock == null || leafBlock.getClass() != bk.class)
        throw new IOException("native grass/leaf block registry entries missing");
      System.setOut(stdout);
      ia.a(grassPixels); jh.a(foliagePixels);

      StringBuilder output = new StringBuilder(); Map<Long, xv> managers = new HashMap<>();
      int climateRows = 0, paletteRows = 0;
      for (Input item : cases) {
        if (item.type().equals("palette")) {
          int grass = ia.a(item.temperature(), item.rainfall()); directGrassPaletteCalls++;
          int foliage = jh.a(item.temperature(), item.rainfall()); directFoliagePaletteCalls++;
          output.append(item.id()).append("\tpalette\t").append(hex32(grass)).append("\t").append(hex32(foliage)).append('\n');
          paletteRows++;
          continue;
        }
        xv climate = manager(item.seed(), managers);
        climate.a(item.x(), item.z(), 1, 1); directClimateCalls++;
        double temperature = climate.a[0], rainfall = climate.b[0];
        NativeWorld world = new NativeWorld(climate);
        int grass = grassBlock.b(world.proxy, item.x(), 64, item.z()); grassCalls++;
        int[] leaves = new int[4];
        for (int metadata = 0; metadata < 4; metadata++) {
          world.metadata = metadata;
          leaves[metadata] = leafBlock.b(world.proxy, item.x(), 64, item.z()); foliageCalls++;
        }
        output.append(item.id()).append("\tclimate\t").append(hex64(Double.doubleToRawLongBits(temperature)))
          .append('\t').append(hex64(Double.doubleToRawLongBits(rainfall))).append('\t').append(hex32(grass));
        for (int color : leaves) output.append('\t').append(hex32(color));
        output.append('\n'); climateRows++;
      }
      stdout.print(output);
      stdout.flush();
      StringBuilder hashes = new StringBuilder();
      for (Class<?> type : audited) hashes.append(type.getName()).append(':').append(classSha(type)).append(',');
      String audit = "COLOR_NATIVE_AUDIT\t1\tjar=" + JAR_SHA + "\tcode_source=" + nativeCodeSource +
        "\tgrass=" + GRASS_SHA + "\tfoliage=" + FOLIAGE_SHA + "\tclasses=" + hashes +
        "\tseed_only_world=Unsafe.allocateInstance(fd_subclass)\tworld_ctor_calls=0\tmanager_ctor_calls=" + managers.size() +
        "\txv_seed_reads=" + seedReads + "\txv_explicit_samples=" + directClimateCalls +
        "\twp_b=" + grassCalls + "\tbk_b=" + foliageCalls + "\tia_explicit_DD=" + directGrassPaletteCalls +
        "\tjh_explicit_DD=" + directFoliagePaletteCalls + "\tcounts_scope=oracle_direct_calls_only" +
        "\tinternal_block_calls=not_instrumented\tclimate_rows=" + climateRows + "\tpalette_rows=" + paletteRows +
        "\tapi=xv.a(IIII):[Lkd;,fields=xv.a:[D/xv.b:[D,wp.b(xp,III):I,bk.b(xp,III):I,ia.a(DD):I,jh.a(DD):I" +
        "\tbootstrap_bytes=" + bootstrap.size() + "\tbootstrap_sha=" + sha(bootstrap.toByteArray()) +
        "\tbootstrap_base64=" + java.util.Base64.getEncoder().encodeToString(bootstrap.toByteArray());
      System.err.println(audit);
    } catch (Throwable failure) {
      System.setOut(System.err);
      failure.printStackTrace(System.err);
      System.exit(1);
    }
  }
}
