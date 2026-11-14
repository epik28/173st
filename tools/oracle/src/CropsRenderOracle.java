import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Capture original world crops render vertices without a GL draw. */
public final class CropsRenderOracle {
  static int meta;
  static String hex(int value) { return String.format(Locale.ROOT,"%08x",value); }
  public static void main(String[] args) throws Exception {
    if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");
    Path jar=Path.of(args[0]).toRealPath();
    String sha=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
    if(!sha.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
    for(Class<?> c:List.of(cv.class,uu.class,nw.class,xp.class)) if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong class origin");
    PrintStream output=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu[] blocks={uu.m[59]};System.setOut(output);
    xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
      Class<?> type=m.getReturnType();if(type==int.class)return m.getName().equals("e")?meta:0;
      if(type==float.class)return 1f;if(type==boolean.class)return false;return null;});
    cv renderer=new cv(world);nw tess=nw.a;Field dataF=nw.class.getDeclaredField("g"),offsetF=nw.class.getDeclaredField("o");dataF.setAccessible(true);offsetF.setAccessible(true);int[] data=(int[])dataF.get(tess);tess.b();
    System.out.println("# native beta 1.7.3 crops render; xyz/uv/color/normal/light words; brightness=1");
    for(int blockIndex=0;blockIndex<blocks.length;blockIndex++)for(meta=0;meta<16;meta++){
      uu block=blocks[blockIndex];int blockId=new int[]{59}[blockIndex];
      offsetF.setInt(tess,0);boolean rendered=renderer.b(block,0,0,0);int n=offsetF.getInt(tess);
      output.println("case\t"+blockId+"\t"+meta+"\t"+(rendered?1:0)+"\t"+(n/8));
      for(int i=0;i<n;i+=8){StringJoiner row=new StringJoiner("\t");row.add("vertex").add(Integer.toString(blockId)).add(Integer.toString(meta)).add(Integer.toString(i/8));for(int j=0;j<8;j++)row.add(hex(data[i+j]));output.println(row);}
    }
  }
}
