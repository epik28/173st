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
  static void fill(W w,int id,int meta,int block,int sky)throws Exception{for(lm c:w.chunks.values()){Arrays.fill(c.b,(byte)0);c.b[(8*16+8)*128+64]=(byte)id;c.f.a(8,64,8,block);c.g.a(8,64,8,sky);}}
  static String nib(lm c,int x,int y,int z){return Integer.toHexString(c.f.a(x&15,y,z&15))+Integer.toHexString(c.g.a(x&15,y,z&15));}
  static String row(W w,String id,int x,int y,int z){int bid=w.a(x,y,z);lm c=w.c(x,z);return id+"\t"+x+"\t"+y+"\t"+z+"\t"+bid+"\t"+(c==null?"--":nib(c,x,y,z))+"\t"+(c==null?"--":Integer.toHexString((c.f.a(x&15,y,z&15)&15)|((c.g.a(x&15,y,z&15)&15)<<4)));}
  static void scenario(W w,String name,int old,int neu,int x,int y,int z,StringBuilder out)throws Exception{fill(w,old,0,0,15);out.append(name+"\tbefore\t"+old+"\t"+neu+"\n");w.d(x,y,z,neu,0);for(int dx=-1;dx<=1;dx++)for(int dz=-1;dz<=1;dz++)out.append(row(w,name+"-q"+(dx+1)+"-"+(dz+1),x+dx,y,z+dz)+"\n");}
  public static void main(String[] a)throws Exception{if(a.length!=2)throw new IOException("usage: JAR AUDIT");Path jar=Path.of(a[0]).toRealPath();if(!sha(jar).equals(JAR_SHA))throw new IOException("wrong client JAR SHA-256");StringBuilder o=new StringBuilder();scenario(world(),"remove",1,0,8,64,8,o);scenario(world(),"place",0,1,8,64,8,o);scenario(world(),"roof",0,1,8,127,8,o);scenario(world(),"edge",0,1,15,64,8,o);byte[] in="light-update-c01-v1\n".getBytes(StandardCharsets.US_ASCII),out=o.toString().getBytes(StandardCharsets.US_ASCII);String audit="{\n  \"jar_sha256\": \""+sha(jar)+"\",\n  \"input_sha256\": \""+shaBytes(in)+"\",\n  \"output_sha256\": \""+shaBytes(out)+"\",\n  \"class\": \"fd\",\n  \"entry\": \"fd.d(IIIII)V\",\n  \"domain\": \"chunks -1..1, y 0..127, queue bounded by 27 loaded chunks\",\n  \"unloaded_edges\": [\"west\",\"east\",\"north\",\"south\"],\n  \"emission\": \"excluded: viewer full-cube edit API does not reach emissive transition\"\n}\n";Files.writeString(Path.of(a[1]),audit);System.out.print(o);}
  static String sha(Path p)throws Exception{return shaBytes(Files.readAllBytes(p));}
  static String shaBytes(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
}
