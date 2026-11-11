import java.io.*;
import java.nio.file.*;
import java.util.*;
/** Calls original GUI item renderer with recording GL and tessellator stubs. */
public final class InventoryCubeOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("InventoryCubeOracle CLIENT_JAR");
    Path jar = Path.of(args[0]).toRealPath();
    if (!ViewOracle.sha(jar).equals(ViewOracle.JAR_SHA)) throw new IOException("wrong JAR");
    for (Class<?> type : List.of(bb.class, cv.class, uu.class, gm.class, iz.class, vg.class, ee.class)) ViewOracle.requireFromJar(type, jar);
    PrintStream out = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    Class.forName("uu"); bb renderer = new bb(); ji textures = new ji();
    System.setOut(out);
    out.println("# jar_sha256\t" + ViewOracle.JAR_SHA);
    for (Class<?> type : List.of(bb.class, cv.class, uu.class, gm.class, iz.class, vg.class, ee.class)) out.println("# class_sha256\t" + type.getName() + "\t" + ViewOracle.classSha(type));
    for (int[] sample : new int[][]{{1,0},{17,0},{17,1},{17,2},{35,0},{35,1},{35,15}}) {
      out.printf("case\t%d\t%d\t10\t20%n",sample[0],sample[1]);
      renderer.a(null,textures,sample[0],sample[1],0,10,20);
    }
  }
}
