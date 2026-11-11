import java.io.*;
import java.nio.file.*;
import java.util.*;
import javax.imageio.ImageIO;

/** World-independent log and wool texture variants from the pinned client. */
public final class BlockMetadataCubeOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 2) throw new IllegalArgumentException("BlockMetadataCubeOracle CLIENT_JAR TERRAIN_PNG");
    Path jar = Path.of(args[0]).toRealPath(), terrain = Path.of(args[1]).toRealPath();
    if (!ViewOracle.sha(jar).equals(ViewOracle.JAR_SHA) || !ViewOracle.sha(terrain).equals(ViewOracle.TERRAIN_SHA)) throw new IOException("wrong pinned input");
    for (Class<?> type : List.of(uu.class, xp.class, ln.class)) ViewOracle.requireFromJar(type, jar);
    var image = ImageIO.read(terrain.toFile());
    if (image == null || image.getWidth() != 256 || image.getHeight() != 256) throw new IOException("invalid terrain dimensions");
    PrintStream report = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    report.println("# jar_sha256\t" + ViewOracle.JAR_SHA);
    report.println("# terrain_sha256\t" + ViewOracle.TERRAIN_SHA);
    for (Class<?> type : List.of(uu.class, xp.class, ln.class, vg.class, ee.class)) {
      ViewOracle.requireFromJar(type, jar);
      report.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
    }
    report.println("# id\tmetadata\trender_type\topaque\tbounds\ttiles\talpha\tstone_to_cube\tcube_to_stone");
    for (int id : new int[]{17, 35}) {
      uu block = uu.m[id]; Class<?> type = block.getClass();
      if (type != (id == 17 ? vg.class : ee.class) ||
          type.getMethod("a", xp.class, int.class, int.class, int.class, int.class).getDeclaringClass() != uu.class ||
          type.getMethod("a", xp.class, int.class, int.class, int.class).getDeclaringClass() != uu.class ||
          type.getMethod("b", xp.class, int.class, int.class, int.class, int.class).getDeclaringClass() != uu.class)
        throw new IOException("unexpected context override for " + id);
      for (int meta = 0; meta < 16; meta++) {
        int[] tiles = new int[6]; StringBuilder alpha = new StringBuilder(), stoneTo = new StringBuilder(), toStone = new StringBuilder();
        for (int face = 0; face < 6; face++) {
          tiles[face] = block.a(ViewOracle.probeWorld(meta, false, ln.a), 0, 0, 0, face);
          alpha.append(ViewOracle.alphaMask(image, tiles[face]));
          stoneTo.append(ViewOracle.visible(uu.m[1], face, block) ? '1' : '0');
          toStone.append(ViewOracle.visible(block, face, uu.m[1]) ? '1' : '0');
        }
        report.printf(Locale.ROOT, "%d\t%d\t%d\t%d\t%.1f,%.1f,%.1f,%.1f,%.1f,%.1f\t%d,%d,%d,%d,%d,%d\t%s\t%s\t%s%n",
          id, meta, block.b(), block.c() ? 1 : 0, block.bs, block.bt, block.bu, block.bv, block.bw, block.bx,
          tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5], alpha, stoneTo, toStone);
      }
    }
  }
}
