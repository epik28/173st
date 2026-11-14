import java.io.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Capture native fence collision bounds; does not create or load a world. */
public final class FenceCollisionOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IllegalArgumentException("CLIENT_JAR");
    Path jar = Path.of(args[0]).toRealPath();
    if (!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalArgumentException("wrong jar");
    for (Class<?> c : List.of(uu.class, jw.class, eq.class)) if (!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IllegalArgumentException("wrong class origin");
    PrintStream out = System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    uu fence = uu.m[85];
    System.setOut(out);
    for (int[] p : new int[][] {{0,0,0},{-3,64,2},{15,127,-16}}) {
      eq box = fence.e(null,p[0],p[1],p[2]);
      out.printf(Locale.ROOT, "%d\t%d\t%d\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f\t%.1f%n",p[0],p[1],p[2],box.a,box.b,box.c,box.d,box.e,box.f);
    }
  }
}
