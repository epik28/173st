import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Capture native block destruction render geometry and vertex colors without GL. */
public final class MiningOverlayOracle {
  public static void main(String[] args) throws Exception {
    if(args.length!=1) throw new IllegalArgumentException("CLIENT_JAR");
    Path jar=Path.of(args[0]).toRealPath();
    if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong jar");
    for(Class<?> c:List.of(cv.class,uu.class,nw.class,xp.class)) if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IllegalArgumentException("wrong class origin");
    PrintStream output=System.out; System.setOut(new PrintStream(OutputStream.nullOutputStream())); uu block=uu.m[1]; System.setOut(output);
    xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
      Class<?> t=m.getReturnType(); if(t==int.class)return 0; if(t==float.class)return 1f; if(t==boolean.class)return false; return null;
    });
    cv renderer=new cv(world); nw tess=nw.a; Field dataF=nw.class.getDeclaredField("g"),offsetF=nw.class.getDeclaredField("o");
    dataF.setAccessible(true); offsetF.setAccessible(true); int[] data=(int[])dataF.get(tess); tess.b();
    System.out.println("stage\tvertex\tx\ty\tz\tu\tv\tcolor");
    for(int stage:new int[]{0,5,9}) {
      offsetF.setInt(tess,0); renderer.a(block,0,0,0,240+stage); int n=offsetF.getInt(tess);
      for(int i=0;i<n;i+=8) System.out.printf(Locale.ROOT,"%d\t%d\t%08x\t%08x\t%08x\t%08x\t%08x\t%08x%n",stage,i/8,data[i],data[i+1],data[i+2],data[i+3],data[i+4],data[i+5]);
    }
  }
}
