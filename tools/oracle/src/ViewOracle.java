import java.io.*;
import java.lang.reflect.Proxy;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.zip.GZIPInputStream;

/** VIEW-001 native block/material mapping oracle. Never constructs a game World. */
public final class ViewOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final String TERRAIN_SHA = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf";
  static final long LIMIT = 64L * 1024 * 1024;
  static int nbtParses, eiConstructors, regionReads, tileCalls, visibilityCalls;
  static final Map<Class<?>, Boolean> WORLD_TILE_OVERRIDES = new HashMap<>();

  static String sha(byte[] bytes) throws Exception {
    return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
  }
  static String sha(Path path) throws Exception { return sha(Files.readAllBytes(path)); }
  static String classSha(Class<?> type) throws Exception {
    try (InputStream in = type.getResourceAsStream("/" + type.getName() + ".class")) {
      if (in == null) throw new IOException("missing class bytes: " + type.getName());
      return sha(in.readAllBytes());
    }
  }
  static void requireFromJar(Class<?> type, Path jar) throws Exception {
    Path source = Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
    if (!source.equals(jar)) throw new IOException("class not loaded from pinned JAR: " + type.getName());
  }
  static byte[] readBounded(InputStream in) throws IOException {
    ByteArrayOutputStream out = new ByteArrayOutputStream(); byte[] buffer = new byte[8192]; int n;
    while ((n = in.read(buffer)) != -1) {
      if ((long)out.size() + n > LIMIT) throw new IOException("decompressed input limit");
      out.write(buffer, 0, n);
    }
    return out.toByteArray();
  }
  static nu parse(byte[] raw) throws Exception {
    DataInputStream input = new DataInputStream(new ByteArrayInputStream(raw));
    ij tag = ij.b(input); nbtParses++;
    if (!(tag instanceof nu root) || input.available() != 0) throw new IOException("expected one complete NBT compound");
    return root;
  }
  static int nibble(byte[] data, int index) {
    int packed = data[index >>> 1] & 255;
    return (index & 1) == 0 ? packed & 15 : packed >>> 4;
  }
  static long stateKey(int id, int metadata) { return ((long)id << 4) | metadata; }
  static String keyId(long key) { return Long.toString((key >>> 4) & 255); }
  static String keyMeta(long key) { return Long.toString(key & 15); }
  static int sceneIndex(int x, int y, int z) { return y * 6400 + z * 80 + x; }
  static int neighborIndex(int x, int y, int z, int face) {
    return switch (face) {
      case 0 -> y == 0 ? -1 : sceneIndex(x, y - 1, z);
      case 1 -> y == 127 ? -1 : sceneIndex(x, y + 1, z);
      case 2 -> z == 0 ? -1 : sceneIndex(x, y, z - 1);
      case 3 -> z == 79 ? -1 : sceneIndex(x, y, z + 1);
      case 4 -> x == 0 ? -1 : sceneIndex(x - 1, y, z);
      case 5 -> x == 79 ? -1 : sceneIndex(x + 1, y, z);
      default -> throw new IllegalArgumentException("face must be 0..5");
    };
  }
  static boolean opaqueNeighbor(uu neighbor) { return neighbor != null && neighbor.c(); }
  static xp probeWorld(int metadata, boolean opaque, ln aboveMaterial) {
    return (xp)Proxy.newProxyInstance(xp.class.getClassLoader(), new Class<?>[]{xp.class}, (proxy, method, args) -> {
      if (method.getName().equals("g")) return opaque;
      if (method.getName().equals("e")) return metadata;
      if (method.getName().equals("f")) return aboveMaterial;
      Class<?> result = method.getReturnType();
      if (result == boolean.class) return false;
      if (result == int.class) return 0;
      if (result == float.class) return 1.0f;
      if (result == double.class) return 0.0d;
      return null;
    });
  }
  static ln material(int id) throws Exception {
    if (id == 0) return ln.a;
    uu block = uu.m[id];
    if (block == null) throw new IOException("scene references unregistered block id " + id);
    Field field = uu.class.getDeclaredField("bA"); field.setAccessible(true);
    return (ln)field.get(block);
  }
  static boolean hasWorldTileOverride(uu block) throws Exception {
    Class<?> type = block.getClass();
    Boolean cached = WORLD_TILE_OVERRIDES.get(type);
    if (cached != null) return cached;
    boolean overridden = type.getMethod("a", xp.class, int.class, int.class, int.class, int.class)
      .getDeclaringClass() != uu.class;
    WORLD_TILE_OVERRIDES.put(type, overridden);
    return overridden;
  }
  static boolean visible(uu block, int face, uu neighbor) {
    visibilityCalls++;
    return block.b(probeWorld(0, opaqueNeighbor(neighbor), ln.a), 0, 0, 0, face);
  }
  static String json(Map<String, ?> values) {
    StringJoiner join = new StringJoiner(",", "{", "}");
    for (var entry : values.entrySet()) join.add("\"" + entry.getKey() + "\":\"" + entry.getValue() + "\"");
    return join.toString();
  }
  static String alphaMask(java.awt.image.BufferedImage image, int tile) throws IOException {
    if (tile < 0 || tile > 255) throw new IOException("native tile outside 0..255: " + tile);
    int x0 = (tile & 15) * 16, y0 = (tile >>> 4) * 16;
    for (int y = y0; y < y0 + 16; y++) for (int x = x0; x < x0 + 16; x++)
      if ((image.getRGB(x, y) >>> 24) != 255) return "0";
    return "1";
  }

  public static void main(String[] args) {
    PrintStream report = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream())); // The JAR's registry prints recipe counts at class init.
    try {
      if (args.length != 3) throw new IllegalArgumentException("ViewOracle CLIENT_JAR INPUT_COPY_DIR TERRAIN_PNG");
      Path jar = Path.of(args[0]).toRealPath(), input = Path.of(args[1]).toRealPath();
      Path terrain = Path.of(args[2]).toRealPath();
      if (!sha(jar).equals(JAR_SHA)) throw new IOException("wrong client JAR SHA-256");
      if (!sha(terrain).equals(TERRAIN_SHA)) throw new IOException("wrong terrain.png SHA-256");
      if (!Files.readString(input.resolve(".world-input-copy")).equals("WORLD-LOAD-001 copied inputs\n") ||
          !Files.readString(input.resolve(".region-input-copy")).equals("REGION-001 copied inputs\n"))
        throw new IOException("hash-verified world copy markers missing");
      for (String name : List.of("level.dat", "r.-1.-1.mcr", "r.-1.0.mcr", "r.0.-1.mcr", "r.0.0.mcr")) {
        Path file = input.resolve(name);
        if (Files.isSymbolicLink(file) || !Files.isRegularFile(file, LinkOption.NOFOLLOW_LINKS) || !file.toRealPath().getParent().equals(input))
          throw new IOException("unsafe/missing copied input: " + name);
      }
      for (Class<?> type : List.of(ij.class, nu.class, ei.class, qj.class, uu.class, cv.class)) requireFromJar(type, jar);
      var image = javax.imageio.ImageIO.read(terrain.toFile());
      if (image == null || image.getWidth() != 256 || image.getHeight() != 256) throw new IOException("terrain.png must be 256x256");

      byte[] packedLevel = Files.readAllBytes(input.resolve("level.dat")), rawLevel;
      try (InputStream gzip = new GZIPInputStream(new ByteArrayInputStream(packedLevel))) { rawLevel = readBounded(gzip); }
      nu levelRoot = parse(rawLevel);
      if (!levelRoot.b("Data")) throw new IOException("level.dat missing Data");
      ei levelInfo = new ei(levelRoot.k("Data")); eiConstructors++;
      int spawnX = levelInfo.c(), spawnY = levelInfo.d(), spawnZ = levelInfo.e();
      int centerChunkX = Math.floorDiv(spawnX, 16), centerChunkZ = Math.floorDiv(spawnZ, 16);
      int firstChunkX = centerChunkX - 2, firstChunkZ = centerChunkZ - 2;
      short[] blocks = new short[80 * 128 * 80];
      Map<Path, qj> regions = new LinkedHashMap<>();
      Field locations = qj.class.getDeclaredField("d"); locations.setAccessible(true);
      try {
        for (int chunkZ = firstChunkZ; chunkZ < firstChunkZ + 5; chunkZ++) {
          for (int chunkX = firstChunkX; chunkX < firstChunkX + 5; chunkX++) {
            String name = "r." + Math.floorDiv(chunkX, 32) + "." + Math.floorDiv(chunkZ, 32) + ".mcr";
            Path path = input.resolve(name); qj region = regions.get(path);
            if (region == null) { region = new qj(path.toFile()); regions.put(path, region); }
            int lx = Math.floorMod(chunkX, 32), lz = Math.floorMod(chunkZ, 32);
            regionReads++;
            try (DataInputStream stream = region.a(lx, lz)) {
              if (stream == null) {
                int slot = lx + lz * 32;
                if (((int[])locations.get(region))[slot] != 0) throw new IOException("native qj absent despite occupied slot");
                throw new IOException("fixed 5x5 scene contains absent chunk " + chunkX + "," + chunkZ);
              }
              byte[] rawChunk = readBounded(stream); nu chunkRoot = parse(rawChunk);
              if (!chunkRoot.b("Level")) throw new IOException("chunk NBT missing Level");
              nu chunk = chunkRoot.k("Level");
              if (chunk.e("xPos") != chunkX || chunk.e("zPos") != chunkZ) throw new IOException("native chunk coordinate mismatch");
              byte[] ids = chunk.j("Blocks"), metadata = chunk.j("Data");
              if (ids.length != 32768 || metadata.length != 16384) throw new IOException("native chunk array length mismatch");
              int ox = (chunkX - firstChunkX) * 16, oz = (chunkZ - firstChunkZ) * 16;
              for (int x = 0; x < 16; x++) for (int z = 0; z < 16; z++) for (int y = 0; y < 128; y++) {
                int source = x * 2048 + z * 128 + y, id = ids[source] & 255;
                int state = (id << 4) | nibble(metadata, source);
                blocks[sceneIndex(ox + x, y, oz + z)] = (short)state;
              }
            }
          }
        }
      } finally { for (qj region : regions.values()) region.b(); }

      TreeSet<Long> states = new TreeSet<>(), mappingCases = new TreeSet<>(), pairs = new TreeSet<>();
      for (int y = 0; y < 128; y++) for (int z = 0; z < 80; z++) for (int x = 0; x < 80; x++) {
        long state = blocks[sceneIndex(x, y, z)] & 0xffffL;
        if ((state >>> 4) == 0) continue;
        states.add(state);
        uu block = uu.m[(int)(state >>> 4)];
        int contextAboveId = -1;
        if (block != null && hasWorldTileOverride(block)) {
          int aboveIndex = neighborIndex(x, y, z, 1);
          contextAboveId = aboveIndex < 0 ? 0 : (blocks[aboveIndex] & 0xffff) >>> 4;
        }
        mappingCases.add((state << 9) | (contextAboveId + 1L));
        for (int face = 0; face < 6; face++) {
          int neighborIndex = neighborIndex(x, y, z, face);
          long neighbor = neighborIndex < 0 ? 0 : blocks[neighborIndex] & 0xffffL;
          pairs.add((state << 15) | ((long)face << 12) | neighbor);
        }
      }
      report.println("#VIEW_NATIVE\t1\t" + spawnX + "\t" + spawnY + "\t" + spawnZ + "\t" + firstChunkX + "\t" + firstChunkZ);
      report.println("#states");
      for (long mappingCase : mappingCases) {
        long state = mappingCase >>> 9;
        int contextAboveId = (int)(mappingCase & 511) - 1;
        int id = (int)(state >>> 4), metadata = (int)(state & 15);
        uu block = uu.m[id];
        if (block == null) { report.println("state\t" + id + "\t" + metadata + "\t" + contextAboveId + "\t-1\t0\t0\t------\t-\t0"); continue; }
        int renderType = block.b(); boolean opaque = block.c(), fullCube = true;
        int[] tiles = new int[6]; StringBuilder alpha = new StringBuilder();
        ln aboveMaterial = contextAboveId < 0 ? ln.a : material(contextAboveId);
        for (int face = 0; face < 6; face++) {
          // Use the renderer-facing virtual method; several blocks override it directly.
          tiles[face] = block.a(probeWorld(metadata, false, aboveMaterial), 0, 0, 0, face); tileCalls++;
          alpha.append(alphaMask(image, tiles[face]));
          if (visible(block, face, uu.m[1])) fullCube = false; // native opaque stone neighbor
        }
        boolean supported = renderType == 0 && opaque && fullCube && alpha.toString().equals("111111");
        report.printf(Locale.ROOT, "state\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%d,%d,%d,%d,%d,%d\t%d%n",
          id, metadata, contextAboveId, renderType, opaque ? 1 : 0, fullCube ? 1 : 0, alpha, tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5], supported ? 1 : 0);
      }
      report.println("#neighbors");
      for (long pair : pairs) {
        int neighbor = (int)(pair & 4095), face = (int)((pair >>> 12) & 7);
        long state = (pair >>> 15) & 4095;
        int id = (int)(state >>> 4), metadata = (int)(state & 15);
        uu block = uu.m[id], other = uu.m[neighbor >>> 4];
        boolean shown = block != null && visible(block, face, other);
        report.printf(Locale.ROOT, "neighbor\t%d\t%d\t%d\t%d\t%d\t%d%n", id, metadata, face, neighbor >>> 4, neighbor & 15, shown ? 1 : 0);
      }
      System.setOut(report);
      Map<String, Object> audit = new LinkedHashMap<>();
      audit.put("jar_sha256", JAR_SHA); audit.put("terrain_sha256", TERRAIN_SHA);
      audit.put("uu_sha256", classSha(uu.class)); audit.put("cv_sha256", classSha(cv.class));
      audit.put("scene_spawn", spawnX + "," + spawnY + "," + spawnZ);
      audit.put("scene_chunks", regionReads); audit.put("unique_states", states.size());
      audit.put("mapping_cases", mappingCases.size());
      audit.put("unique_neighbor_pairs", pairs.size()); audit.put("native_calls_ij", nbtParses);
      audit.put("native_calls_ei", eiConstructors); audit.put("native_calls_qj", regionReads);
      audit.put("native_calls_tile", tileCalls); audit.put("native_calls_visibility", visibilityCalls);
      audit.put("world_aware_tile_cases", mappingCases.size() - states.size());
      audit.put("native_world_or_chunk_loads", 0); audit.put("native_renderer_vertex_calls", 0);
      System.err.println(json(audit));
    } catch (Throwable error) {
      System.setOut(report); error.printStackTrace(System.err); System.exit(1);
    }
  }
}
