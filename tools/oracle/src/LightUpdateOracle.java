import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Bounded adapter around the Beta 1.7.3 in-memory block update entry point. */
public final class LightUpdateOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final sun.misc.Unsafe U=unsafe();
  static sun.misc.Unsafe unsafe(){try{Field f=sun.misc.Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);return (sun.misc.Unsafe)f.get(null);}catch(Exception e){throw new Error(e);}}
  static void obj(Object o,Class<?> c,String n,Object v)throws Exception{Field f=c.getDeclaredField(n);U.putObject(o,U.objectFieldOffset(f),v);}
  static void integer(Object o,Class<?> c,String n,int v)throws Exception{Field f=c.getDeclaredField(n);U.putInt(o,U.objectFieldOffset(f),v);}
  static final class W extends fd {
    final Map<Long,lm> chunks=new HashMap<>();
    W(){super((wt)null,"",(xa)null,0L);}
    public lm c(int x,int z){return chunks.get((((long)(x>>4))<<32)^((z>>4)&0xffffffffL));}
    public int a(int x,int y,int z){lm c=c(x,z);return c==null?0:c.a(x&15,y,z&15);}
  }
  static long key(int x,int z){return (((long)x)<<32)^(z&0xffffffffL);}
  static W world()throws Exception{W w=(W)U.allocateInstance(W.class);obj(w,W.class,"chunks",new HashMap<Long,lm>());obj(w,fd.class,"u",new ArrayList<>());xa p=xa.a(0);p.e();obj(w,fd.class,"t",p);obj(w,fd.class,"x",new ei(0L,"light-update"));integer(w,fd.class,"f",0);for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++){lm c=new lm(w,cx,cz);c.b=new byte[32768];c.f=new wi(32768);c.g=new wi(32768);c.e=new wi(32768);w.chunks.put(key(cx,cz),c);}return w;}
  static void fill(W w,int id,int meta,int block,int sky,int x,int y,int z)throws Exception{for(lm c:w.chunks.values()){Arrays.fill(c.b,(byte)0);if(c.j==(x>>4)&&c.k==(z>>4))c.b[((x&15)*16+(z&15))*128+y]=(byte)id;c.f.a(x&15,y,z&15,block);c.g.a(x&15,y,z&15,sky);}}
  static String nib(lm c,int x,int y,int z){return Integer.toHexString(c.f.a(x&15,y,z&15))+Integer.toHexString(c.g.a(x&15,y,z&15));}
  static String row(W w,String id,int x,int y,int z){int bid=w.a(x,y,z);lm c=w.c(x,z);return id+"\t"+x+"\t"+y+"\t"+z+"\t"+bid+"\t"+(c==null?"--":nib(c,x,y,z))+"\t"+(c==null?"--":Integer.toHexString((c.f.a(x&15,y,z&15)&15)|((c.g.a(x&15,y,z&15)&15)<<4)));}
  static void scenario(W w,String name,int old,int neu,int x,int y,int z,String qs,StringBuilder out)throws Exception{fill(w,old,0,0,15,x,y,z);out.append(name+"\tbefore\t"+old+"\t"+neu+"\n");w.e(x,y,z,neu,0);w.d(x,y,z,neu,0);for(String q:qs.split(";")){String[] p=q.split(",");out.append(row(w,name+"-"+p[0],Integer.parseInt(p[1]),Integer.parseInt(p[2]),Integer.parseInt(p[3]))+"\n");}}
  public static void main(String[] a)throws Exception{if(a.length!=2)throw new IOException("usage: JAR AUDIT");Path jar=Path.of(a[0]).toRealPath();if(!sha(jar).equals(JAR_SHA))throw new IOException("wrong client JAR SHA-256");byte[] in=System.in.readAllBytes();StringBuilder o=new StringBuilder();PrintStream real=System.out;System.setOut(new PrintStream(new ByteArrayOutputStream()));try{for(String line:new String(in,StandardCharsets.US_ASCII).split("\\n")){if(line.isBlank())continue;String[] p=line.split("\\t",-1);if(p.length!=7||!p[0].equals("case"))throw new IOException("invalid input case");scenario(world(),p[1],Integer.parseInt(p[2]),Integer.parseInt(p[3]),Integer.parseInt(p[4]),Integer.parseInt(p[5]),Integer.parseInt(p[6].split("\\|",2)[0]),p[6].split("\\|",2)[1],o);}}finally{System.setOut(real);}byte[] out=o.toString().getBytes(StandardCharsets.US_ASCII);String audit="{\n  \"jar_sha256\": \""+sha(jar)+"\",\n  \"input_sha256\": \""+shaBytes(in)+"\",\n  \"output_sha256\": \""+shaBytes(out)+"\",\n  \"class\": \"fd\",\n  \"entry\": \"fd.d(IIIII)V\",\n  \"domain\": \"y 0..127; loaded synthetic chunks only; no unloaded-edge claim\",\n  \"emission\": \"excluded: viewer full-cube edit API does not reach emissive transition\"\n}\n";Files.writeString(Path.of(a[1]),audit);System.out.print(o);}
  static String sha(Path p)throws Exception{return shaBytes(Files.readAllBytes(p));}
  static String shaBytes(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
}
