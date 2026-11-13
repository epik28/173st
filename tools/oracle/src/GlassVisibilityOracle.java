import java.io.*;import java.lang.reflect.*;import java.nio.file.*;import java.security.*;import java.util.*;
/** Direct native glass-face visibility against air, stone, and another glass block. */
public final class GlassVisibilityOracle {
 static int neighbor;
 public static void main(String[] args)throws Exception {
  if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();
  if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  for(Class<?> c:List.of(uu.class,xp.class,fk.class)) if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong class origin");
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu slab=uu.m[20];System.setOut(out);
  xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{Class<?> t=m.getReturnType();if(t==int.class)return m.getName().equals("a")?neighbor:0;if(t==boolean.class)return m.getName().equals("g") && neighbor==1;if(t==float.class)return 1f;return null;});
  for(neighbor=0;neighbor<=20;neighbor++)if(neighbor==0||neighbor==1||neighbor==20)for(int face=0;face<6;face++)out.println(neighbor+"\t"+face+"\t"+(slab.b(world,0,0,0,face)?1:0));
 }
}
