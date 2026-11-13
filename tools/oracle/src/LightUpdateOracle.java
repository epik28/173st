import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Bounded adapter around the Beta 1.7.3 in-memory block/light update path. */
public final class LightUpdateOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final sun.misc.Unsafe U=unsafe();
  static sun.misc.Unsafe unsafe(){try{Field f=sun.misc.Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);return (sun.misc.Unsafe)f.get(null);}catch(Exception e){throw new Error(e);}}
  static void obj(Object o,Class<?> c,String n,Object v)throws Exception{Field f=c.getDeclaredField(n);U.putObject(o,U.objectFieldOffset(f),v);}
  static void integer(Object o,Class<?> c,String n,int v)throws Exception{Field f=c.getDeclaredField(n);U.putInt(o,U.objectFieldOffset(f),v);}
  static long key(int x,int z){return (((long)x)<<32)^(z&0xffffffffL);}
  static final class W extends fd {
    Map<Long,lm> chunks;
    W(){super((wt)null,"",(xa)null,0L);}
    public lm c(int chunkX,int chunkZ){return chunks.get(key(chunkX,chunkZ));}
    public int a(int x,int y,int z){lm c=c(x>>4,z>>4);return c==null?0:c.a(x&15,y,z&15);}
  }
  static final class C implements cl {
    final W w; C(W w){this.w=w;}
    public boolean a(int x,int z){return w.chunks.containsKey(key(x,z));}
    public lm b(int x,int z){return w.chunks.get(key(x,z));}
    public lm c(int x,int z){return w.chunks.get(key(x,z));}
    public void a(cl c,int x,int z){}
    public boolean a(boolean b,yb y){return false;} public boolean a(){return false;} public boolean b(){return false;}
    public String c(){return "synthetic-light";}
  }
  static W world()throws Exception{
    W w=(W)U.allocateInstance(W.class); obj(w,W.class,"chunks",new HashMap<Long,lm>());
    obj(w,fd.class,"u",new ArrayList<>()); obj(w,fd.class,"C",new ArrayList<>()); obj(w,fd.class,"v",new C(w));
    xa p=xa.a(0); p.e(); obj(w,fd.class,"t",p); obj(w,fd.class,"x",new ei(0L,"light-update")); integer(w,fd.class,"f",0);
    for(int cx=-1;cx<=1;cx++) for(int cz=-1;cz<=1;cz++){
      lm c=new lm(w,cx,cz); c.b=new byte[32768]; c.f=new wi(32768); c.g=new wi(32768); c.e=new wi(32768); w.chunks.put(key(cx,cz),c);
    }
    // Native chunk initialization derives the height map and direct sky nibble array
    // from the all-air block arrays. Do not overwrite these internally consistent values.
    for(lm c:w.chunks.values()) c.c();
    while(w.j()){}
    return w;
  }
  static int sky(lm c,int x,int y,int z){return c.f.a(x&15,y,z&15);}
  static int block(lm c,int x,int y,int z){return c.g.a(x&15,y,z&15);}
  static int combined(W w,int x,int y,int z){return w.a(x,y,z,false);}
  static int meta(lm c,int x,int y,int z){return c.e.a(x&15,y,z&15);}
  static String domainRows(W w,String name,int old,int oldMeta,int x,int y,int z,String digest){
    StringBuilder rows=new StringBuilder();
    rows.append("domain\t").append(name)
      .append("\tchunks_x=-1,0,1\tchunks_z=-1,0,1\ty=0..127")
      .append("\tdefault_block=0\tdefault_meta=0\tdefault_block_light=0")
      .append("\tdefault_sky=0@y0;15@y1..127(native-lm.c-all-air)")
      .append("\tdefault_height=0(native-lm.c-all-air)")
      .append("\tmaterial=uu.q[id],uu.s[id]")
      .append("\tinit=xa.a(0);lm.c();fd.j-quiescent")
      .append("\told_override=").append(old==0?"-":x+","+y+","+z+","+old+","+oldMeta+";fd.a+fd.j-quiescent")
      .append("\tdigest=").append(digest).append('\n');
    for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++){
      lm c=w.c(cx,cz);
      for(int ly=0;ly<128;ly++)for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++){
        int id=c.a(lx,ly,lz),md=c.e.a(lx,ly,lz),bl=c.g.a(lx,ly,lz),sk=c.f.a(lx,ly,lz);
        int defaultSky=ly==0?0:15;
        if(id!=0||md!=0||bl!=0||sk!=defaultSky){
          int wx=cx*16+lx,wz=cz*16+lz;
          rows.append("state\t").append(name).append('\t').append(wx).append('\t').append(ly).append('\t').append(wz)
            .append('\t').append(id).append('\t').append(md).append('\t').append(bl).append('\t').append(sk).append('\n');
        }
      }
      for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++){
        int h=c.h[lz*16+lx];
        if(h!=0)rows.append("height\t").append(name).append('\t').append(cx*16+lx).append('\t').append(cz*16+lz).append('\t').append(h).append('\n');
      }
    }
    return rows.toString();
  }
  static String stateDigest(W w)throws Exception{
    MessageDigest d=MessageDigest.getInstance("SHA-256");
    List<lm> cs=new ArrayList<>(w.chunks.values());cs.sort(Comparator.comparingInt((lm c)->c.j).thenComparingInt(c->c.k));
    for(lm c:cs){d.update((byte)c.j);d.update((byte)c.k);d.update(c.b);d.update(c.e.a);d.update(c.f.a);d.update(c.g.a);d.update(c.h);}
    return HexFormat.of().formatHex(d.digest());
  }
  static String row(W w,String name,String phase,int x,int y,int z){
    lm c=w.c(x>>4,z>>4);if(c==null)return name+"\t"+phase+"\t"+x+"\t"+y+"\t"+z+"\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED\tUNLOADED";int id=w.a(x,y,z);return name+"\t"+phase+"\t"+x+"\t"+y+"\t"+z+"\t"+id+"\t"+meta(c,x,y,z)+"\t"+uu.q[id]+"\t"+uu.s[id]+"\t"+sky(c,x,y,z)+"\t"+block(c,x,y,z)+"\t"+combined(w,x,y,z);
  }
  static void settle(W w,int x,int y,int z)throws Exception{
    w.a(eb.a,x-1,y-1,z-1,x+1,y+1,z+1);
    int guard=0; while(w.j()){if(++guard>1000)throw new IOException("native light queue did not settle");}
  }
  static void scenario(String name,int old,int oldMeta,int neu,int newMeta,int x,int y,int z,String qs,StringBuilder out)throws Exception{
    W w=world();
    if(old!=0){w.a(x,y,z,old,oldMeta);settle(w,x,y,z);}
    String[] queries=qs.split(";");
    List<String> unloaded=new ArrayList<>();
    for(String q:queries){String[] p=q.split(",");int qx=Integer.parseInt(p[0]),qy=Integer.parseInt(p[1]),qz=Integer.parseInt(p[2]);if(w.c(qx>>4,qz>>4)==null)unloaded.add(qx+","+qy+","+qz);}
    String initialDigest=stateDigest(w);
    out.append("case\t"+name+"\t"+old+"\t"+oldMeta+"\t"+neu+"\t"+newMeta+"\t"+x+"\t"+y+"\t"+z+"\tinitial="+initialDigest+"\tunloaded="+(unloaded.isEmpty()?"-":String.join(";",unloaded))+"\n");
    out.append(domainRows(w,name,old,oldMeta,x,y,z,initialDigest));
    for(String q:queries){String[] p=q.split(",");int qx=Integer.parseInt(p[0]),qy=Integer.parseInt(p[1]),qz=Integer.parseInt(p[2]);out.append(row(w,name,"before",qx,qy,qz)+"\n");}
    w.a(x,y,z,neu,newMeta); settle(w,x,y,z);
    if(w.a(x,y,z)!=neu)throw new IOException("native fd.a block mutation failed for "+name+": expected "+neu+" got "+w.a(x,y,z));
    out.append("final\t"+name+"\t"+stateDigest(w)+"\n");
    for(String q:queries){String[] p=q.split(",");int qx=Integer.parseInt(p[0]),qy=Integer.parseInt(p[1]),qz=Integer.parseInt(p[2]);out.append(row(w,name,"after",qx,qy,qz)+"\n");}
  }
  public static void main(String[] a)throws Exception{
    if(a.length!=3)throw new IOException("usage: JAR AUDIT SOURCE"); Path jar=Path.of(a[0]).toRealPath(), source=Path.of(a[2]).toRealPath();
    if(!sha(jar).equals(JAR_SHA))throw new IOException("wrong client JAR SHA-256");
    byte[] in=System.in.readAllBytes(); StringBuilder o=new StringBuilder(); PrintStream real=System.out;
    System.setOut(new PrintStream(new ByteArrayOutputStream()));
    try { for(String line:new String(in,StandardCharsets.US_ASCII).split("\\n")){
      if(line.isBlank()||line.startsWith("#"))continue; String[] p=line.split("\\t",-1);
      if(p.length!=9||!p[0].equals("case"))throw new IOException("invalid input case");
      String[] xyz=p[8].split("\\|",2); scenario(p[1],Integer.parseInt(p[2]),Integer.parseInt(p[3]),Integer.parseInt(p[4]),Integer.parseInt(p[5]),Integer.parseInt(p[6]),Integer.parseInt(p[7]),Integer.parseInt(xyz[0]),xyz[1],o);
    }} finally { System.setOut(real); }
    byte[] output=o.toString().getBytes(StandardCharsets.US_ASCII);
    String audit="{\n"+
      "  \"jar_sha256\": \""+sha(jar)+"\",\n"+
      "  \"source_sha256\": \""+sha(source)+"\",\n"+
      "  \"input_sha256\": \""+shaBytes(in)+"\",\n"+
      "  \"output_sha256\": \""+shaBytes(output)+"\",\n"+
      "  \"class_sha256\": {\n"+
      "    \"fd\": \""+classSha(fd.class)+"\",\n"+
      "    \"lm\": \""+classSha(lm.class)+"\",\n"+
      "    \"wi\": \""+classSha(wi.class)+"\",\n"+
      "    \"uu\": \""+classSha(uu.class)+"\",\n"+
      "    \"eb\": \""+classSha(eb.class)+"\",\n"+
      "    \"st\": \""+classSha(st.class)+"\"\n"+
      "  },\n"+
      "  \"entry\": \"fd.a(IIIII)Z block setter; fd.a(eb, bounds); fd.j(); st.a(fd)\",\n"+
      "  \"seed_schema\": {\n"+
      "    \"loaded_chunk_coordinates\": {\"x\": [-1, 0, 1], \"z\": [-1, 0, 1]},\n"+
      "    \"block_domain\": {\"y_min\": 0, \"y_max_inclusive\": 127},\n"+
      "    \"initial_block_id\": 0,\n"+
      "    \"initial_metadata\": 0,\n"+
      "    \"initial_block_light_nibble\": 0,\n"+
      "    \"initial_sky_light\": \"native lm.c() all-air profile: 0 at y=0, 15 at y=1..127; native.tsv state rows encode deviations\",\n"+
      "    \"old_id_override\": \"for nonzero old ID, set only the target cell with metadata 0 and settle native light queue before initial digest and before-phase queries\",\n"+
      "    \"provider\": \"xa.a(0) with native e() initialization; seed 0; synthetic world key light-update\",\n"+
      "    \"initialization\": \"all nine chunks allocate zeroed block, metadata, sky-light, block-light and height-map arrays; call lm.c() for each, then drain fd.j() to quiescence; native.tsv domain/state/height rows encode defaults and all deviations\",\n"+
      "    \"native_tsv_encoding\": \"one domain row per case; state rows encode every nondefault block/meta/block-light/sky cell; height rows encode every nondefault height cell; domain digest is the complete native initial state digest\",\n"+
      "    \"material_properties\": \"native uu.q[id] opacity and uu.s[id] emission\"\n"+
      "  },\n"+
      "  \"domain\": \"synthetic loaded chunks x/z -1..1, y 0..127; initial all air metadata 0; opacity/emission from uu.q/uu.s; native lm.c() initializes height-map/direct sky\",\n"+
      "  \"light_layout\": \"lm.f sky; lm.g block; lm.c(IIII)I combined max(sky-subtraction,block)\",\n"+
      "  \"emission\": \"excluded: viewer full-cube edit API does not reach an emissive transition\"\n"+
      "}\n";
    Files.writeString(Path.of(a[1]),audit); System.out.write(output);
  }
  static String sha(Path p)throws Exception{return shaBytes(Files.readAllBytes(p));}
  static String shaBytes(byte[] b)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b));}
  static String classSha(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName()+".class")){return shaBytes(in.readAllBytes());}}
}
