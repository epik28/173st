import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Native ID-31 selection-boundary rays in fresh in-memory chunks only. */
public final class GrassRayOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final double MIN=(double)(0.5f-0.4f), MAX=(double)(0.5f+0.4f);
  record Ray(String id,double sx,double sy,double sz,double ex,double ey,double ez){}
  static List<Ray> rays(){double y=10.4;return List.of(
    new Ray("x_mid_inside",-1,y,.5,2,y,.5),
    new Ray("x_above_height",-1,10.85,.5,2,10.85,.5),
    new Ray("x_min_exact_along_z",MIN,y,-1,MIN,y,2),
    new Ray("x_min_below_along_z",Math.nextDown(MIN),y,-1,Math.nextDown(MIN),y,2),
    new Ray("x_max_exact_along_z",MAX,y,-1,MAX,y,2),
    new Ray("x_max_above_along_z",Math.nextUp(MAX),y,-1,Math.nextUp(MAX),y,2),
    new Ray("y_bottom_exact",.5,9,.5,.5,12,.5),
    new Ray("y_top_exact",.5,12,.5,.5,9,.5),
    new Ray("vertical_x_below",Math.nextDown(MIN),9,.5,Math.nextDown(MIN),12,.5),
    new Ray("vertical_x_min_exact",MIN,9,.5,MIN,12,.5),
    new Ray("vertical_x_max_exact",MAX,9,.5,MAX,12,.5),
    new Ray("vertical_x_above",Math.nextUp(MAX),9,.5,Math.nextUp(MAX),12,.5),
    new Ray("diagonal_inside",-1,10.5,-1,2,10.5,2),
    new Ray("diagonal_above",-1,10.85,-1,2,10.85,2));}

  static final class Provider implements wt {
    final Map<Long,lm> chunks=new HashMap<>();
    static long key(int x,int z){return ((long)x<<32)^(z&0xffffffffL);}
    public ei c(){return null;} public void b(){}
    public bf a(xa d){return new bf(){
      public lm a(fd w,int x,int z){return chunks.computeIfAbsent(key(x,z),k->new lm(w,new byte[32768],x,z));}
      public void a(fd w,lm c){chunks.put(key(c.j,c.k),c);} public void b(fd w,lm c){chunks.put(key(c.j,c.k),c);}
      public void a(){} public void b(){}
    };}
    public void a(ei d,List l){} public void a(ei d){} public File a(String s){return null;}
  }
  static Object alloc(Class<?> c)throws Exception{Field f=Class.forName("sun.misc.Unsafe").getDeclaredField("theUnsafe");f.setAccessible(true);return ((sun.misc.Unsafe)f.get(null)).allocateInstance(c);}
  static fd world(Provider p)throws Exception{
    fd seed=(fd)alloc(fd.class);Field f=fd.class.getDeclaredField("w");f.setAccessible(true);f.set(seed,p);
    f=fd.class.getDeclaredField("t");f.setAccessible(true);f.set(seed,xa.a(0));
    f=fd.class.getDeclaredField("x");f.setAccessible(true);f.set(seed,new ei(0L,"memory"));
    return fd.class.getConstructor(fd.class,xa.class).newInstance(seed,xa.a(0));
  }
  static void put(fd w,int x,int y,int z,int id)throws Exception{if(!w.b(x>>4,z>>4).a(x&15,y,z&15,id,0))throw new IOException("native chunk refused setup");}
  static String bits(double d){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(d));}
  static String input(Ray r){return String.join("\t",r.id,Double.toString(r.sx),Double.toString(r.sy),Double.toString(r.sz),Double.toString(r.ex),Double.toString(r.ey),Double.toString(r.ez));}
  static String output(Ray r)throws Exception{
    fd w=world(new Provider());put(w,0,10,0,31);
    vf h=w.a(bt.a(r.sx,r.sy,r.sz),bt.a(r.ex,r.ey,r.ez),false,false);
    if(h==null)return r.id+"\tmiss\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-";
    return String.join("\t",r.id,"block",Integer.toString(h.b),Integer.toString(h.c),Integer.toString(h.d),Integer.toString(h.e),Double.toString(h.f.a),Double.toString(h.f.b),Double.toString(h.f.c),bits(h.f.a),bits(h.f.b),bits(h.f.c));
  }
  static byte[] capture(Path jar,boolean inputs)throws Exception{
    if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("pinned client JAR mismatch");
    PrintStream report=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));Class.forName("uu");
    ByteArrayOutputStream bytes=new ByteArrayOutputStream();PrintStream out=new PrintStream(bytes,true,"US-ASCII");
    out.println(inputs?"#BLOCK_GRASS_RAY_INPUT\t1\t"+JAR_SHA:"#BLOCK_GRASS_RAY_NATIVE\t1\t"+JAR_SHA);
    out.println(inputs?"#id\tsx\tsy\tsz\tex\tey\tez":"#id\tkind\tx\ty\tz\tside\tpx\tpy\tpz\tpx_bits\tpy_bits\tpz_bits");
    for(Ray r:rays())out.println(inputs?input(r):output(r));out.flush();System.setOut(report);return bytes.toByteArray();
  }
  static String sha(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
  static String classSha(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){return sha(in.readAllBytes());}}
  static Map<String,String> hashes(Path jar)throws Exception{Map<String,String> m=new LinkedHashMap<>();for(Class<?> c:new Class<?>[]{fd.class,bt.class,vf.class,uu.class,ru.class}){if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("class not from pinned JAR: "+c.getName());m.put(c.getSimpleName(),classSha(c));}return m;}
  static String json(Map<String,String> m){StringJoiner j=new StringJoiner(",","{","}");m.forEach((k,v)->j.add("\""+k+"\":\""+v+"\""));return j.toString();}
  public static void main(String[] args)throws Exception{
    if(args.length!=2||!args[1].equals("--rays")&&!args[1].equals("--inputs"))throw new IllegalArgumentException("GrassRayOracle CLIENT_JAR (--rays|--inputs)");Path jar=Path.of(args[0]).toRealPath();boolean inputs=args[1].equals("--inputs");
    byte[] first=capture(jar,inputs),second=capture(jar,inputs);if(!Arrays.equals(first,second))throw new AssertionError("native grass rays differ within process");System.out.write(first);
    System.err.println("{\"jar_sha256\":\""+JAR_SHA+"\",\"classes\":"+json(hashes(jar))+",\"ray_method\":\"fd.a(bt,bt,boolean,boolean):vf\",\"shape_dispatch\":\"uu.a(fd,int,int,int,bt,bt):vf\",\"shape_handler\":\"ru extends wb; ru.<init>(II) with block id 31 calls a(0.5f-0.4f,0,0.5f-0.4f,0.5f+0.4f,0.8f,0.5f+0.4f)\",\"native_cases\":"+rays().size()+",\"captures\":2,\"repeat_identical\":true,\"world_constructors\":0,\"save_access\":false}");
  }
}
