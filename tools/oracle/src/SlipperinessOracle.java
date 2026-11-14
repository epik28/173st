import java.io.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;
/** Capture all registered Beta block slipperiness values. */
public final class SlipperinessOracle {
 public static void main(String[] args) throws Exception {
  if(args.length != 1) throw new IllegalArgumentException("CLIENT_JAR");
  Path jar=Path.of(args[0]).toRealPath();
  if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  if(!Path.of(uu.class.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong class origin");
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu[] blocks=uu.m;System.setOut(out);
  for(int id=1;id<blocks.length;id++)if(blocks[id]!=null)out.printf(Locale.ROOT,"%d\t%08x%n",id,Float.floatToRawIntBits(blocks[id].bB));
 }
}
