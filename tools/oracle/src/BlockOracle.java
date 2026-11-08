import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Native, read-only ray trace corpus against the pinned Beta 1.7.3 client. */
public final class BlockOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final String[] NAMES = {"fd", "vf", "bt", "uu", "ob", "pj", "iz", "gm"};
  static String sha(byte[] b) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b)); }
  static String classSha(Class<?> c) throws Exception {
    try (InputStream in = c.getResourceAsStream("/" + c.getName().replace('.', '/') + ".class")) {
      if (in == null) throw new IOException("missing class bytes " + c.getName());
      return sha(in.readAllBytes());
    }
  }
  static Object zero(Class<?> c) {
    if (c == boolean.class) return false;
    if (c == int.class) return 0;
    if (c == long.class) return 0L;
    if (c == float.class) return 0f;
    if (c == double.class) return 0d;
    return null;
  }
  static final class MemoryProvider implements wt {
    final Map<Long, lm> chunks = new HashMap<>();
    static long key(int x, int z) { return ((long)x << 32) ^ (z & 0xffffffffL); }
    @Override public ei c() { return null; }
    @Override public void b() {}
    @Override public bf a(xa dim) { return new bf() {
      public lm a(fd w, int x, int z) { return chunks.computeIfAbsent(key(x,z), k -> new lm(w,new byte[32768],x,z)); }
      public void a(fd w,lm c) { chunks.put(key(c.j,c.k),c); }
      public void b(fd w,lm c) { chunks.put(key(c.j,c.k),c); }
      public void a() {} public void b() {}
    }; }
    @Override public void a(ei data,List list) {}
    @Override public void a(ei data) {}
    @Override public File a(String name) { return null; }
  }
  record Ray(String id, String shape, double sx, double sy, double sz, double ex, double ey, double ez, int stopLiquid) {}
  static List<Ray> rays() { return List.of(
    new Ray("face_west","stone",-2,.5,.5,2,.5,.5,0), new Ray("face_east","stone",2,.5,.5,-2,.5,.5,0),
    new Ray("face_down","stone",.5,-2,.5,.5,2,.5,0), new Ray("face_up","stone",.5,2,.5,.5,-2,.5,0),
    new Ray("face_north","stone",.5,.5,-2,.5,.5,2,0), new Ray("face_south","stone",.5,.5,2,.5,.5,-2,0),
    new Ray("miss","stone",-2,2,.5,2,2,.5,0), new Ray("negative_voxel","stone",-2,.5,.5,2,.5,.5,0),
    new Ray("tie_corner","stone",-1,-1,-1,2,2,2,0),
    new Ray("endpoint_before","stone",-2,.5,.5,-Math.ulp(1d),.5,.5,0),
    new Ray("endpoint_face","stone",-2,.5,.5,0,.5,.5,0),
    new Ray("shape_slab","slab",-2,.25,.5,2,.25,.5,0),
    new Ray("liquid_stop_false","water",-2,.5,.5,2,.5,.5,0),
    new Ray("liquid_stop_true","water",-2,.5,.5,2,.5,.5,1),
    new Ray("range_before","stone",0,.5,.5,4.999,.5,.5,0),
    new Ray("range_at","stone",0,.5,.5,5,.5,.5,0),
    new Ray("range_after","stone",0,.5,.5,5.001,.5,.5,0)); }
  static fd newWorld(MemoryProvider provider) throws Exception {
    // fd's copy constructor skips new-world spawn generation and accepts a
    // provider whose only chunks are explicitly allocated by this oracle.
    fd seed = (fd)UnsafeHolder.allocate(fd.class);
    Field providerField=fd.class.getDeclaredField("w"); providerField.setAccessible(true); providerField.set(seed,provider);
    Field dimensionField=fd.class.getDeclaredField("t"); dimensionField.setAccessible(true); dimensionField.set(seed,xa.a(0));
    Field dataField=fd.class.getDeclaredField("x"); dataField.setAccessible(true); dataField.set(seed,new ei(0L,"memory"));
    Constructor<fd> c=fd.class.getConstructor(fd.class,xa.class);
    return c.newInstance(seed,xa.a(0));
  }
  static final class UnsafeHolder {
    static Object allocate(Class<?> type)throws Exception {
      Field f=Class.forName("sun.misc.Unsafe").getDeclaredField("theUnsafe");f.setAccessible(true);
      return ((sun.misc.Unsafe)f.get(null)).allocateInstance(type);
    }
  }
  static void put(fd w, int x,int y,int z,int id) {
    lm chunk = w.b(x >> 4,z >> 4);
    if (!chunk.a(x & 15,y,z & 15,id,0)) throw new IllegalStateException("native chunk refused block setup");
  }
  static void scene(fd w, Ray ray) {
    if (ray.id.equals("negative_voxel")) { put(w,-1,0,0,1); return; }
    if (ray.id.startsWith("range_")) { put(w,5,0,0,1); return; }
    if (ray.id.equals("shape_slab")) { put(w,0,0,0,44); return; }
    if (ray.id.startsWith("liquid_")) { put(w,0,0,0,8); return; }
    put(w,0,0,0,ray.shape.equals("slab")?44:1);
  }
  static String raw(double d) { return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(d)); }
  static String hit(String id, vf h) {
    if (h == null) return id+"\tlive\tmiss\t-\t-\t-\t-\t-\t-\t-\t-\t-\t-";
    bt p=h.f;
    return String.join("\t",id,"live",h.a==jg.a?"block":"other",Integer.toString(h.b),Integer.toString(h.c),Integer.toString(h.d),Integer.toString(h.e),String.valueOf(p.a),String.valueOf(p.b),String.valueOf(p.c),raw(p.a),raw(p.b),raw(p.c));
  }
  public static void main(String[] args) throws Exception {
    if(args.length!=1) throw new IllegalArgumentException("BlockOracle CLIENT_JAR");
    Path jar=Path.of(args[0]).toRealPath();
    if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA)) throw new IOException("wrong pinned JAR SHA-256");
    Class<?>[] classes={fd.class,vf.class,bt.class,uu.class,ob.class,pj.class,iz.class,gm.class};
    Map<String,String> hashes=new LinkedHashMap<>();
    for(int i=0;i<classes.length;i++){
      Class<?> c=classes[i];
      if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("class not loaded from pinned jar: "+NAMES[i]);
      hashes.put(NAMES[i],classSha(c));
    }
    PrintStream report=System.out;
    System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    report.println("#BLOCK_NATIVE\t1\t"+JAR_SHA);
    report.println("#id\tsource\tkind\tx\ty\tz\tside\tpx\tpy\tpz\tpx_bits\tpy_bits\tpz_bits");
    // Suppress only registry bootstrap chatter; ray results remain machine-readable.
    for(Ray ray:rays()){
      MemoryProvider provider=new MemoryProvider(); fd world=newWorld(provider); scene(world,ray);
      bt start=bt.a(ray.sx,ray.sy,ray.sz), end=bt.a(ray.ex,ray.ey,ray.ez);
      vf result=world.a(start,end,ray.stopLiquid!=0,false);
      report.println(hit(ray.id,result));
    }
    System.setOut(report);
    System.err.println("{\"jar_sha256\":\""+JAR_SHA+"\",\"classes\":"+jsonHashes(hashes)+",\"ray_method\":\"fd.a(bt,bt,boolean,boolean):vf\",\"shape_dispatch\":\"uu.a(fd,int,int,int,bt,bt):vf\",\"reach\":\"ob.b():5.0f; pj extends ob\",\"world_constructors\":0,\"native_cases\":"+rays().size()+",\"actions\":\"not_probed\"}");
  }
  static String jsonHashes(Map<String,String> hashes) {
    StringJoiner j=new StringJoiner(",","{","}");
    hashes.forEach((k,v)->j.add("\""+k+"\":\""+v+"\""));
    return j.toString();
  }
}
