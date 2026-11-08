import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Captures native cv.b -> nw.a packed color values; it performs no GL draw/flush. */
public final class VertexColorOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final float[] BRIGHTNESS={0.25f,0.5f,Float.intBitsToFloat(0x3e85b05c)};
  static final float[] TINT={0.123f,0.456f,0.789f};
  static float currentBrightness;
  static int nativeCalls, brightnessCalls, packedVertices;
  static String sha(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
  static byte[] classBytes(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){if(in==null)throw new IllegalStateException("missing class resource: "+c.getName());return in.readAllBytes();}}
  static String source(Class<?> c)throws Exception{return Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().toString();}
  static String bits(int i){return String.format(Locale.ROOT,"%08x",i);}
  static String fbits(float f){return bits(Float.floatToRawIntBits(f));}
  static String face(int blockId,boolean overlay,int group){
    String[] order=(blockId==2&&overlay)
      ?new String[]{"down","up","north","north","south","south","west","west","east","east"}
      :new String[]{"down","up","north","south","west","east"};
    if(group>=order.length)throw new IllegalStateException("unexpected native face group "+group);
    return order[group];
  }
  static String layer(int blockId,boolean overlay,int group){return blockId==2&&overlay&&group>=2&&(group&1)==1?"overlay":"base";}
  static Object world(){return Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
    if(m.getName().equals("a")&&m.getReturnType()==int.class)return 0;
    Class<?> r=m.getReturnType();
    if(r==float.class){if(m.getParameterCount()==4)brightnessCalls++;return currentBrightness;}
    if(r==boolean.class)return false;if(r==int.class)return 0;if(r==double.class)return 1d;return null;
  });}
  static void checkJar(Path jar)throws Exception{
    if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IllegalStateException("wrong client JAR SHA-256");
    for(Class<?> c:new Class<?>[]{xp.class,uu.class,cv.class,nw.class,net.minecraft.client.Minecraft.class})
      if(!Path.of(source(c)).toRealPath().equals(jar))throw new IllegalStateException("class not loaded from pinned JAR: "+c.getName());
  }
  public static void main(String[] args)throws Exception{
    if(args.length!=1)throw new IllegalArgumentException("VertexColorOracle CLIENT_JAR");
    Path jar=Path.of(args[0]).toRealPath();checkJar(jar);
    PrintStream out=System.out;ByteArrayOutputStream boot=new ByteArrayOutputStream();
    System.setOut(new PrintStream(boot,true,StandardCharsets.UTF_8));
    uu grass=uu.m[2], stone=uu.m[1];System.setOut(out);
    out.println("#VERTEX_COLOR_NATIVE\t1\tjar="+JAR_SHA+"\txp="+sha(classBytes(xp.class))+"\tuu="+sha(classBytes(uu.class))+"\tcv="+sha(classBytes(cv.class))+"\tnw="+sha(classBytes(nw.class))+"\tminecraft="+sha(classBytes(net.minecraft.client.Minecraft.class)));
    out.println("#sources\txp="+sha(Files.readAllBytes(Path.of(source(xp.class))))+"\tuu="+sha(Files.readAllBytes(Path.of(source(uu.class))))+"\tcv="+sha(Files.readAllBytes(Path.of(source(cv.class))))+"\tnw="+sha(Files.readAllBytes(Path.of(source(nw.class)))));
    byte[] bb=boot.toByteArray();out.println("#bootstrap\tbytes="+bb.length+"\tsha256="+sha(bb)+"\tbase64="+Base64.getEncoder().encodeToString(bb));
    out.println("#setup\txp dynamic proxy: neighbor integer=0, boolean=false, brightness float=case input; no world/chunk load; no GL draw/flush");
    out.println("#input\tbrightness=3e800000,3f000000,3e85b05c\ttint="+fbits(TINT[0])+","+fbits(TINT[1])+","+fbits(TINT[2])+"\tblocks=1,2\tlayers=0,1");
    cv renderer=new cv((xp)world());nw tess=nw.a;
    Field dataF=nw.class.getDeclaredField("g"),offsetF=nw.class.getDeclaredField("o");dataF.setAccessible(true);offsetF.setAccessible(true);int[] data=(int[])dataF.get(tess);
    tess.b();
    for(int bi:new int[]{1,2})for(float brightness:BRIGHTNESS)for(boolean overlay:new boolean[]{false,true}){
      currentBrightness=brightness;cv.a=overlay;offsetF.setInt(tess,0);
      uu block=bi==1?stone:grass;nativeCalls++;
      boolean rendered=renderer.b(block,0,64,0,TINT[0],TINT[1],TINT[2]);int after=offsetF.getInt(tess);
      out.println("case\t"+bi+"\t"+fbits(brightness)+"\t"+(overlay?1:0)+"\t"+(rendered?1:0)+"\t"+(after/8));
      for(int i=0;i<after;i+=8){int group=i/48;out.println("vertex\t"+bi+"\t"+fbits(brightness)+"\t"+(overlay?1:0)+"\t"+face(bi,overlay,group)+"\t"+layer(bi,overlay,group)+"\t"+(i/8)+"\t"+fbits(TINT[0])+"\t"+fbits(TINT[1])+"\t"+fbits(TINT[2])+"\t"+bits(data[i+5]));packedVertices++;}
    }
    out.println("#calls\tcv_b="+nativeCalls+"\txp_brightness="+brightnessCalls+"\tpacked_vertices="+packedVertices+"\tgl_draw=0\tgl_flush=0");out.flush();
  }
}
