import java.io.*;import java.lang.reflect.*;import java.nio.file.*;import java.security.*;import java.util.*;
/** Direct native ls.p() ladder lookup with a recording synthetic world. */
public final class LadderPresenceOracle {
 static int id, x, y, z, calls;
 static final class W extends fd {
  W(){super((wt)null,"unused",0L);}
  @Override public int a(int px,int py,int pz){LadderPresenceOracle.x=px;LadderPresenceOracle.y=py;LadderPresenceOracle.z=pz;calls++;return id;}
 }
 static final class E extends ls {E(fd world){super(world);}}
 public static void main(String[] args)throws Exception {
  if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();
  if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  for(Class<?> c:List.of(ls.class,sn.class,fd.class,eq.class,uu.class))if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong class origin");
  Field f=sun.misc.Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);sun.misc.Unsafe u=(sun.misc.Unsafe)f.get(null);
  W world=(W)u.allocateInstance(W.class);Field entities=fd.class.getDeclaredField("u");entities.setAccessible(true);entities.set(world,Collections.emptyList());
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));E entity=new E(world);uu ladder=uu.m[65];System.setOut(out);if(ladder.bn!=65)throw new IllegalStateException("wrong ladder id");
  double[][] cases={{8.5,1.0,4.69,1.0},{-0.1,64.9,-1.1,64.2},{2.9,10.9,3.2,8.0},{0.0,-0.1,0.0,-0.01}};
  for(int i=0;i<cases.length;i++)for(int block:new int[]{0,65,66}){double[] c=cases[i];entity.e(c[0],c[1],c[2]);entity.aM=c[0];entity.aN=c[1];entity.aO=c[2];entity.aW.b=c[3];id=block;calls=0;boolean present=entity.p();if(calls!=1)throw new IllegalStateException("unexpected query count");out.println(i+"\t"+block+"\t"+x+"\t"+y+"\t"+z+"\t"+(present?1:0));}
 }
}
