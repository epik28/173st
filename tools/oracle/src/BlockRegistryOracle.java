import java.io.*;
import java.nio.file.*;
import java.util.*;

/** Independent metadata-zero inventory of the pinned client's registered blocks. */
public final class BlockRegistryOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("BlockRegistryOracle CLIENT_JAR");
    Path jar = Path.of(args[0]).toRealPath();
    if (!ViewOracle.sha(jar).equals(ViewOracle.JAR_SHA)) throw new IOException("wrong client JAR SHA-256");
    ViewOracle.requireFromJar(uu.class, jar);
    ViewOracle.requireFromJar(xp.class, jar);
    ViewOracle.requireFromJar(ln.class, jar);
    PrintStream report = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    report.println("# jar_sha256\t" + ViewOracle.JAR_SHA);
    for (Class<?> type : List.of(uu.class, xp.class, ln.class))
      report.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
    report.println("# id\tclass\tclass_sha256\trender_type\topaque\tworld_tile_override\ttiles_metadata0_air_above");
    for (int id = 0; id < uu.m.length; id++) {
      uu block = uu.m[id];
      if (block == null) continue;
      Class<?> type = block.getClass();
      ViewOracle.requireFromJar(type, jar);
      int[] tiles = new int[6];
      for (int face = 0; face < 6; face++)
        tiles[face] = block.a(ViewOracle.probeWorld(0, false, ln.a), 0, 0, 0, face);
      report.printf(Locale.ROOT, "%d\t%s\t%s\t%d\t%d\t%d\t%d,%d,%d,%d,%d,%d%n",
        id, type.getName(), ViewOracle.classSha(type), block.b(), block.c() ? 1 : 0,
        ViewOracle.hasWorldTileOverride(block) ? 1 : 0,
        tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5]);
    }
  }
}
