import java.io.*;
import java.nio.file.*;
import java.util.*;
/** Native item stack variants and the associated block face textures. */
public final class InventoryVariantOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("InventoryVariantOracle CLIENT_JAR");
    Path jar = Path.of(args[0]).toRealPath();
    if (!ViewOracle.sha(jar).equals(ViewOracle.JAR_SHA)) throw new IOException("wrong JAR");
    for (Class<?> type : List.of(uu.class, gm.class, iz.class, vg.class, ee.class)) ViewOracle.requireFromJar(type, jar);
    PrintStream report = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    report.println("# jar_sha256\t" + ViewOracle.JAR_SHA);
    for (Class<?> type : List.of(uu.class, gm.class, iz.class, vg.class, ee.class))
      report.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
    report.println("# id\tdamage\tstack_limit\tmax_durability\tblock_faces");
    for (int id : new int[]{17, 35}) for (int damage = 0; damage < 16; damage++) {
      iz stack = new iz(id, 1, damage);
      int[] tiles = new int[6];
      for (int face = 0; face < 6; face++) tiles[face] = uu.m[id].a(face, stack.i());
      report.printf(Locale.ROOT, "%d\t%d\t%d\t%d\t%d,%d,%d,%d,%d,%d%n", id, stack.i(), stack.c(), stack.j(), tiles[0], tiles[1], tiles[2], tiles[3], tiles[4], tiles[5]);
    }
  }
}
