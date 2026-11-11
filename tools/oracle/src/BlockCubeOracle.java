import java.io.*;
import java.nio.file.*;
import java.util.*;
import javax.imageio.ImageIO;

/** Reproducible evidence for the viewer's additional metadata-zero cubes. */
public final class BlockCubeOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 2) throw new IllegalArgumentException("BlockCubeOracle CLIENT_JAR TERRAIN_PNG");
    Path jar = Path.of(args[0]).toRealPath(), terrain = Path.of(args[1]).toRealPath();
    if (!ViewOracle.sha(jar).equals(ViewOracle.JAR_SHA)) throw new IOException("wrong client JAR SHA-256");
    if (!ViewOracle.sha(terrain).equals(ViewOracle.TERRAIN_SHA)) throw new IOException("wrong terrain SHA-256");
    for (Class<?> type : List.of(uu.class, xp.class, ln.class)) ViewOracle.requireFromJar(type, jar);
    var image = ImageIO.read(terrain.toFile());
    if (image == null || image.getWidth() != 256 || image.getHeight() != 256) throw new IOException("invalid terrain dimensions");
    PrintStream report = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    report.println("# jar_sha256\t" + ViewOracle.JAR_SHA);
    report.println("# terrain_sha256\t" + ViewOracle.TERRAIN_SHA);
    for (Class<?> type : List.of(uu.class, xp.class, ln.class))
      report.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
    report.println("# id\trender_type\topaque\tbounds\ttiles\talpha\tstone_to_cube\tcube_to_stone\tcube_to_each_added");
    int[] ids = {4, 5, 22, 41, 42, 45, 47, 48, 57, 58, 80, 82, 87, 89};
    for (int id : ids) {
      uu block = uu.m[id];
      if (block == null) throw new IOException("missing block " + id);
      Class<?> type = block.getClass();
      ViewOracle.requireFromJar(type, jar);
      if (type.getMethod("a", xp.class, int.class, int.class, int.class, int.class).getDeclaringClass() != uu.class ||
          type.getMethod("a", xp.class, int.class, int.class, int.class).getDeclaringClass() != uu.class ||
          type.getMethod("b", xp.class, int.class, int.class, int.class, int.class).getDeclaringClass() != uu.class)
        throw new IOException("context override for " + id);
      report.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
      int[] tiles = new int[6]; StringBuilder alpha = new StringBuilder(), stoneTo = new StringBuilder(), toStone = new StringBuilder(), pairs = new StringBuilder();
      for (int face = 0; face < 6; face++) {
        tiles[face] = block.a(ViewOracle.probeWorld(0, false, ln.a), 0, 0, 0, face);
        for (int meta = 1; meta < 16; meta++)
          if (block.a(ViewOracle.probeWorld(meta, false, ln.a), 0, 0, 0, face) != tiles[face])
            throw new IOException("metadata texture variation for " + id);
        alpha.append(ViewOracle.alphaMask(image, tiles[face]));
        stoneTo.append(ViewOracle.visible(uu.m[1], face, block) ? '1' : '0');
        toStone.append(ViewOracle.visible(block, face, uu.m[1]) ? '1' : '0');
      }
      for (int other : ids) {
        if (pairs.length() > 0) pairs.append(',');
        for (int face = 0; face < 6; face++) pairs.append(ViewOracle.visible(block, face, uu.m[other]) ? '1' : '0');
      }
      report.printf(Locale.ROOT, "%d\t%d\t%d\t%.1f,%.1f,%.1f,%.1f,%.1f,%.1f\t%d,%d,%d,%d,%d,%d\t%s\t%s\t%s\t%s%n",
        id, block.b(), block.c() ? 1 : 0, block.bs, block.bt, block.bu, block.bv, block.bw, block.bx,
        tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5], alpha, stoneTo, toStone, pairs);
    }
  }
}
