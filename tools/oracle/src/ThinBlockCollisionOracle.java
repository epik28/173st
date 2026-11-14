import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;
import sun.misc.Unsafe;
/** Native collision boxes by metadata for ladder and snow. No world file is loaded. */
public final class ThinBlockCollisionOracle {
 static final class World extends fd {
  int metadata;
  World() { super((wt)null,"unused",0L); }
  @Override public int e(int x,int y,int z) { return metadata; }
 }
 public static void main(String[] args)throws Exception {
  if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");
  Path jar=Path.of(args[0]).toRealPath();
  if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  for(Class<?> c:List.of(uu.class,fd.class,eq.class))if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong class origin");
  Field unsafeField=Unsafe.class.getDeclaredField("theUnsafe");unsafeField.setAccessible(true);Unsafe unsafe=(Unsafe)unsafeField.get(null);
  World world=(World)unsafe.allocateInstance(World.class);
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu[] blocks={uu.m[65],uu.m[78]};System.setOut(out);
  for(uu block:blocks) for(int metadata=0;metadata<16;metadata++) {
    if(block.bn==65 && (metadata<2 || metadata>5))continue;
    world.metadata=metadata;
    // Renderer updates snow's metadata-dependent bounds before collision queries.
    block.a((xp)world,0,0,0);
    eq b=block.e(world,0,0,0);
    if(b==null)out.printf("%d\t%d\tnone%n",block.bn,metadata);
    else out.printf(Locale.ROOT,"%d\t%d\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f\t%.4f%n",block.bn,metadata,b.a,b.b,b.c,b.d,b.e,b.f);
  }
 }
}
