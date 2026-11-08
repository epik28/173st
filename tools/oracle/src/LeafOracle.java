import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Calls the pinned Beta leaf and flat-grass renderer paths without GL draw/flush. */
public final class LeafOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static int leafCalls, setterCalls, visibilityCalls, opacityCalls, tileCalls, grassCalls, vertexCalls, collisionCalls;
  static int neighborId;
  static final SortedMap<String,Integer> WORLD_CALLS = new TreeMap<>();
  static String suiteInput;
  static float[] grassRgb;
  static String sha(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
  static byte[] classBytes(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){if(in==null)throw new IllegalStateException("missing class resource: "+c.getName());return in.readAllBytes();}}
  static String source(Class<?> c)throws Exception{return Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().toString();}
  static String sourceSha(Class<?> c)throws Exception{return sha(Files.readAllBytes(Path.of(source(c))));}
  static Object world(){return Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
    String signature=m.getName()+"("+String.join(",",Arrays.stream(m.getParameterTypes()).map(Class::getSimpleName).toList())+")";
    WORLD_CALLS.merge(signature,1,Integer::sum);
    switch(m.getName()){
      case "a": if(m.getReturnType()==int.class){return (a!=null&&a.length>0&&a[0] instanceof Integer)?neighborId:0;}
      case "g": if(m.getReturnType()==boolean.class){return neighborId!=0&&((uu.m[neighborId]).c());}
      case "h": if(m.getReturnType()==boolean.class)return false;
      default: break;
    }
    Class<?> r=m.getReturnType(); if(r==boolean.class)return false;if(r==int.class)return 0;if(r==float.class)return 1f;if(r==double.class)return 1d;return null;
  });}
  static String bits(int i){return String.format(Locale.ROOT,"%08x",i);}
  static String fbits(float f){return bits(Float.floatToRawIntBits(f));}
  static void checkJar(Path jar)throws Exception{if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IllegalStateException("wrong client JAR SHA-256");for(Class<?> c:new Class<?>[]{bk.class,nr.class,uu.class,cv.class,nw.class,net.minecraft.client.Minecraft.class})if(!Path.of(source(c)).toRealPath().equals(jar))throw new IllegalStateException("class not loaded from pinned JAR: "+c.getName());}
  public static void main(String[] args)throws Exception{
    if(args.length!=1)throw new IllegalArgumentException("LeafOracle CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();checkJar(jar);
    try(BufferedReader reader=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){suiteInput=reader.readLine();if(!"suite\tLEAF-001-C01\tdefault=constructor\tfancy=0,1\tmetadata=0..15\tfaces=0..5\tneighbors=0,1,18,20\tgrass_rgb=3e4ccccd,3ecccccd,3f19999a".equals(suiteInput)||reader.readLine()!=null)throw new IllegalArgumentException("expected the single frozen LEAF-001-C01 suite input row");String[] rgb=suiteInput.split("\\t")[7].substring("grass_rgb=".length()).split(",");grassRgb=new float[]{Float.intBitsToFloat((int)Long.parseLong(rgb[0],16)),Float.intBitsToFloat((int)Long.parseLong(rgb[1],16)),Float.intBitsToFloat((int)Long.parseLong(rgb[2],16))};}
    PrintStream out=System.out;ByteArrayOutputStream boot=new ByteArrayOutputStream();System.setOut(new PrintStream(boot,true,StandardCharsets.UTF_8));uu grass=uu.m[2];bk leaves=(bk)uu.m[18];uu stone=uu.m[1];uu glass=uu.m[20];System.setOut(out);
    Object world=world();
    out.println("#LEAF_NATIVE\t1\tjar="+JAR_SHA+"\tbk="+sha(classBytes(bk.class))+"\tnr="+sha(classBytes(nr.class))+"\tuu="+sha(classBytes(uu.class))+"\tcv="+sha(classBytes(cv.class))+"\tnw="+sha(classBytes(nw.class))+"\tminecraft="+sha(classBytes(net.minecraft.client.Minecraft.class)));
    out.println("#sources\tbk="+sourceSha(bk.class)+"\tnr="+sourceSha(nr.class)+"\tuu="+sourceSha(uu.class)+"\tcv="+sourceSha(cv.class)+"\tnw="+sourceSha(nw.class)+"\tminecraft="+sourceSha(net.minecraft.client.Minecraft.class));
    byte[] bb=boot.toByteArray();out.println("#bootstrap\tbytes="+bb.length+"\tsha256="+sha(bb)+"\tbase64="+Base64.getEncoder().encodeToString(bb));
    out.println("#methods\tleaf= bk.a(int,int), bk.c(), bk.b(xp,int,int,int,int), uu.f(fd,int,int,int)\tgrass=cv.b(uu,int,int,int,float,float,float)\tpacked=nw.g:xyz/uv/color");
    out.println("#setup\txp is a dynamic proxy supplying neighbor IDs/opacity and brightness 1; no world/chunk load; no GL draw/flush");
    out.println("#input\t"+suiteInput);
    out.println("#expected-render-state\tMinecraft constructor bytecode: GL11.glDepthFunc(515=GL_LEQUAL); GL11.glAlphaFunc(516=GL_GREATER,0.1f)");
    boolean defaultOpaque=leaves.c();opacityCalls++;int defaultTile=leaves.a(0,0);tileCalls++;out.println("default\t"+(defaultOpaque?1:0)+"\t"+defaultTile);
    for(boolean fancy:new boolean[]{false,true}){
      leaves.a(fancy);setterCalls++;
      for(int meta=0;meta<16;meta++){
        boolean opaque=leaves.c();opacityCalls++;
        for(int face=0;face<6;face++){int tile=leaves.a(face,meta);tileCalls++;leafCalls++;out.println("leaf\t"+(fancy?1:0)+"\t"+meta+"\t"+face+"\t"+(opaque?1:0)+"\t"+tile);}
      }
      for(int n:new int[]{0,1,18,20})for(int face=0;face<6;face++){
        neighborId=n;boolean visible=leaves.b((xp)world,0,64,0,face);visibilityCalls++;
        out.println("visible\t"+(fancy?1:0)+"\t"+n+"\t"+face+"\t"+(visible?1:0));
      }
    }
    // Collision query uses the original block method with a null world; record if this
    // API path accepts it rather than manufacturing unit-cube bounds in the adapter.
    leaves.a(true);try{eq box=leaves.f(null,0,64,0);collisionCalls++;out.println("collision\tOK\t"+Double.toHexString(box.a)+"\t"+Double.toHexString(box.b)+"\t"+Double.toHexString(box.c)+"\t"+Double.toHexString(box.d)+"\t"+Double.toHexString(box.e)+"\t"+Double.toHexString(box.f));}catch(Throwable t){out.println("collision\tERROR\t"+t.getClass().getName()+"\t"+String.valueOf(t.getMessage()));}
    // Invoke original flat cv.b with a synthetic world at brightness 1 and a fixed tint.
    // Only xp calls used by this path are supplied; no world load or GL operation occurs.
    neighborId=0;cv renderer=new cv((xp)world);nw tess=nw.a;Field dataF=nw.class.getDeclaredField("g"),offsetF=nw.class.getDeclaredField("o");dataF.setAccessible(true);offsetF.setAccessible(true);int[] data=(int[])dataF.get(tess);
    tess.b();
    for(boolean overlay:new boolean[]{false,true}){
      cv.a=overlay;int before=(int)offsetF.get(tess);boolean rendered=renderer.b(grass,0,64,0,grassRgb[0],grassRgb[1],grassRgb[2]);grassCalls++;
      int after=(int)offsetF.get(tess);out.println("grass\t"+(overlay?1:0)+"\t"+(rendered?1:0)+"\t"+(after-before)/8);
      for(int i=before;i<after;i+=8){out.println("vertex\t"+(overlay?1:0)+"\t"+((i-before)/8)+"\t"+bits(data[i])+"\t"+bits(data[i+1])+"\t"+bits(data[i+2])+"\t"+bits(data[i+3])+"\t"+bits(data[i+4])+"\t"+bits(data[i+5]));vertexCalls++;}
    }
    for(var call:WORLD_CALLS.entrySet())out.println("#world_call\t"+call.getKey()+"\t"+call.getValue());
    out.println("#calls\tleaf_api="+leafCalls+"\tsetters="+setterCalls+"\tvisibility="+visibilityCalls+"\topacity="+opacityCalls+"\ttile="+tileCalls+"\tgrass_renderer="+grassCalls+"\tvertices="+vertexCalls+"\tcollision="+collisionCalls+"\tgl_draw=0\tgl_flush=0");out.flush();
  }
}
