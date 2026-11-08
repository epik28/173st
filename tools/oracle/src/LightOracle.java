import java.io.*;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Direct adapter to the pinned Beta 1.7.3 saved-light/provider methods. */
public final class LightOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final int MAX_BYTES = 16 * 1024 * 1024, MAX_ROWS = 100_000;
  static int directChunkLightCalls, directWorldBrightnessCalls, directAngleCalls, directSkyCalls;
  static int directProviderFactoryCalls, directProviderTableCalls, directMetadataTimeSets;
  static int directEmissionReads;
  static final sun.misc.Unsafe U = unsafe();

  static sun.misc.Unsafe unsafe() {
    try { Field f=sun.misc.Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); return (sun.misc.Unsafe)f.get(null); }
    catch (Exception e) { throw new ExceptionInInitializerError(e); }
  }
  static final class SyntheticWorld extends fd {
    lm selectedChunk;
    SyntheticWorld() { super((wt)null, "", (xa)null, 0L); }
    @Override public int a(int x,int y,int z) { return 0; }
    @Override public lm c(int x,int z) { return selectedChunk; }
  }
  record Row(String id,String kind,int dimension,int block,int sky,int subtraction,int minimum,long time,int partialBits,int blockId) {}
  static String sha(byte[] v) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(v)); }
  static String sha(Path p) throws Exception { return sha(Files.readAllBytes(p)); }
  static String hex32(int x) { return String.format(Locale.ROOT,"%08x",x); }
  static String json(String s) { return "\""+s.replace("\\","\\\\").replace("\"","\\\"")+"\""; }
  static byte[] classBytes(Class<?> c) throws Exception {
    try (InputStream in=c.getResourceAsStream("/"+c.getName()+".class")) { if(in==null) throw new IOException("missing class bytes: "+c.getName()); return in.readAllBytes(); }
  }
  static String codeSource(Class<?> c) throws Exception { return Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().toString(); }
  static void requireJar(Class<?> c,Path jar) throws Exception { if(!Path.of(codeSource(c)).equals(jar.toRealPath())) throw new IOException("class not loaded from configured client JAR: "+c.getName()+" source="+codeSource(c)); }
  static long signed(String s,long lo,long hi) throws IOException {
    if(!s.matches("0|-?[1-9][0-9]*")) throw new IOException("noncanonical signed decimal: "+s);
    try { long v=Long.parseLong(s); if(v<lo||v>hi) throw new IOException("integer out of range: "+s); return v; }
    catch(NumberFormatException e) { throw new IOException("integer out of range: "+s,e); }
  }
  static List<Row> parse(byte[] data) throws Exception {
    if(data.length==0||data.length>MAX_BYTES||data[data.length-1]!='\n') throw new IOException("input must be nonempty, <=16MiB and LF-terminated");
    for(byte b:data) if((b&128)!=0||b=='\r') throw new IOException("input must be ASCII with LF endings");
    String[] lines=new String(data,StandardCharsets.US_ASCII).substring(0,data.length-1).split("\n",-1);
    if(lines.length>MAX_ROWS) throw new IOException("row limit exceeded");
    Set<String> ids=new HashSet<>(); List<Row> rows=new ArrayList<>(lines.length);
    for(int n=0;n<lines.length;n++) {
      String[] c=lines[n].split("\t",-1); if(c.length<2||!c[0].matches("[A-Za-z0-9_.:-]{1,96}")||!ids.add(c[0])) throw new IOException("line "+(n+1)+": invalid/duplicate id");
      if(c[1].equals("sample")&&c.length==7) {
        int d=(int)signed(c[2],-1,0); if(d!=-1&&d!=0) throw new IOException("dimension must be -1 or 0");
        int b=(int)signed(c[3],0,15),s=(int)signed(c[4],0,15),sub=(int)signed(c[5],0,15),min=(int)signed(c[6],0,15);
        rows.add(new Row(c[0],c[1],d,b,s,sub,min,0,0,0));
      } else if(c[1].equals("time")&&c.length==5) {
        int d=(int)signed(c[2],-1,0); if(d!=-1&&d!=0) throw new IOException("dimension must be -1 or 0");
        long t=signed(c[3],Long.MIN_VALUE,Long.MAX_VALUE); if(!c[4].matches("[0-9a-f]{8}")) throw new IOException("partial must be lowercase raw f32 hex");
        int bits=(int)Long.parseLong(c[4],16); float p=Float.intBitsToFloat(bits); if(!Float.isFinite(p)||p<0f||p>1f) throw new IOException("partial outside finite 0..1");
        rows.add(new Row(c[0],c[1],d,0,0,0,0,t,bits,0));
      } else if(c[1].equals("emission")&&c.length==3) {
        int id=(int)signed(c[2],0,255); rows.add(new Row(c[0],c[1],0,0,0,0,0,0,0,id));
      } else throw new IOException("line "+(n+1)+": invalid kind or field count");
    }
    return rows;
  }
  static void setInt(Object o,Class<?> owner,String name,int v) throws Exception { Field f=owner.getDeclaredField(name); U.putInt(o,U.objectFieldOffset(f),v); }
  static void setObject(Object o,Class<?> owner,String name,Object v) throws Exception { Field f=owner.getDeclaredField(name); U.putObject(o,U.objectFieldOffset(f),v); }
  static SyntheticWorld makeWorld(int dim) throws Exception {
    SyntheticWorld w=(SyntheticWorld)U.allocateInstance(SyntheticWorld.class);
    xa provider=xa.a(dim); directProviderFactoryCalls++; provider.e(); directProviderTableCalls++;
    ei info=new ei(0L,"light-oracle-synthetic");
    setObject(w,fd.class,"t",provider); setObject(w,fd.class,"x",info);
    lm chunk=new lm(w,0,0); chunk.f=new wi(32768); chunk.g=new wi(32768); w.selectedChunk=chunk;
    setInt(w,fd.class,"f",0); // clear sky subtraction until set for each sample
    // fd weather state i/j/k/l is left at Java's zero default by Unsafe allocation.
    return w;
  }
  static void putNibble(wi a,int value) { a.a(0,64,0,value); }
  static String run(List<Row> rows,byte[] boot) throws Exception {
    Map<Integer,SyntheticWorld> worlds=new HashMap<>(); StringBuilder out=new StringBuilder();
    for(Row r:rows) {
      if(r.kind.equals("emission")) { out.append(r.id).append("\temission\t").append(uu.s[r.blockId]).append('\n'); directEmissionReads++; continue; }
      SyntheticWorld w=worlds.computeIfAbsent(r.dimension,d->{ try{return makeWorld(d);}catch(Exception e){throw new RuntimeException(e);} });
      if(r.kind.equals("sample")) {
        putNibble(w.selectedChunk.f,r.sky); putNibble(w.selectedChunk.g,r.block); setInt(w,fd.class,"f",r.subtraction);
        int combined=w.selectedChunk.c(0,64,0,r.subtraction); directChunkLightCalls++;
        float brightness=w.a(0,64,0,r.minimum); directWorldBrightnessCalls++;
        out.append(r.id).append("\tsample\t").append(combined).append('\t').append(hex32(Float.floatToRawIntBits(brightness))).append('\n');
      } else {
        w.x.a(r.time); directMetadataTimeSets++; xa p=w.t;
        float partial=Float.intBitsToFloat(r.partialBits);
        float angle=p.a(r.time,partial); directAngleCalls++;
        int sky=w.a(partial); directSkyCalls++;
        out.append(r.id).append("\ttime\t").append(hex32(Float.floatToRawIntBits(angle))).append('\t').append(sky).append('\n');
      }
    }
    return out.toString();
  }
  static String audit(Path jar,byte[] input,byte[] output,byte[] boot,List<Row> rows) throws Exception {
    Class<?>[] types={fd.class,xa.class,rh.class,wd.class,lm.class,wi.class,uu.class,ei.class,xp.class,in.class};
    String[] names={"fd","xa","rh","wd","lm","wi","uu","ei","xp","in"};
    String source=codeSource(fd.class); for(Class<?> t:types) requireJar(t,jar); for(Class<?> t:types) if(!codeSource(t).equals(source)) throw new IOException("native classes have mixed code sources: "+t.getName());
    StringBuilder b=new StringBuilder("{\n  \"jar_sha256\": ").append(json(sha(Files.readAllBytes(jar)))).append(",\n  \"code_source\": ").append(json(source)).append(",\n  \"classes\": {");
    for(int i=0;i<types.length;i++) b.append(i==0?"\n":" ,\n").append("    ").append(json(names[i])).append(": ").append(json(sha(classBytes(types[i]))));
    b.append("\n  },\n  \"bootstrap_base64\": ").append(json(Base64.getEncoder().encodeToString(boot)))
      .append(",\n  \"bootstrap_sha256\": ").append(json(sha(boot)))
      .append(",\n  \"input_sha256\": ").append(json(sha(input))).append(",\n  \"output_sha256\": ").append(json(sha(output)))
      .append(",\n  \"direct_counts\": {\n    \"lm_c_explicit\": ").append(directChunkLightCalls)
      .append(",\n    \"fd_a_IIII\": ").append(directWorldBrightnessCalls).append(",\n    \"xa_a_JF\": ").append(directAngleCalls)
      .append(",\n    \"fd_a_F\": ").append(directSkyCalls).append(",\n    \"xa_factory\": ").append(directProviderFactoryCalls)
      .append(",\n    \"xa_e\": ").append(directProviderTableCalls).append(",\n    \"ei_a_J\": ").append(directMetadataTimeSets)
      .append(",\n    \"uu_s_reads\": ").append(directEmissionReads)
      .append("\n  },\n  \"internal_calls\": \"not instrumented; fd.a(IIII) normal path is known to call lm.c once per sample; fd.a(F) invokes provider smoothing internally\",\n  \"constructors\": {\"fd\": 0, \"synthetic_fd\": 0, \"lm\": ").append(worldsCount(rows))
      .append("},\n  \"weather_state\": \"clear: fd.i/j/k/l zero-initialized; fd.f set per sample\",\n  \"provider_dimensions\": \"0=Overworld/rh,-1=Nether/wd\",\n  \"special_light_query_ids_excluded\": [44,53,60,67],\n  \"light_used_static_flag\": \"lm.a may be set by original saved-light query; outside pure numerical output\"\n}\n");
    return b.toString();
  }
  static int worldsCount(List<Row> rows) { return (int)rows.stream().filter(r->!r.kind.equals("emission")).map(r->r.dimension).distinct().count(); }
  static void verifyNativeSurface() throws Exception {
    if (lm.class.getMethod("c",int.class,int.class,int.class,int.class).getDeclaringClass()!=lm.class ||
        fd.class.getMethod("a",int.class,int.class,int.class,int.class).getDeclaringClass()!=fd.class ||
        fd.class.getMethod("a",float.class).getDeclaringClass()!=fd.class ||
        xa.class.getMethod("a",long.class,float.class).getDeclaringClass()!=xa.class ||
        ei.class.getMethod("a",long.class).getDeclaringClass()!=ei.class)
      throw new IOException("unexpected native method owner/signature");
    xa.class.getDeclaredMethod("e"); // base method; wd overrides it in the native JAR
    if(wd.class.getDeclaredMethod("e").getDeclaringClass()!=wd.class)
      throw new IOException("unexpected Nether brightness-table override owner");
    if (uu.s.length!=256) throw new IOException("native emission table is not 256 entries");
    for(int level:uu.s) if(level<0||level>15) throw new IOException("native emission level out of range: "+level);
  }
  public static void main(String[] args) {
    try {
      if(args.length!=2) throw new IOException("usage: LightOracle CLIENT_JAR AUDIT_JSON");
      Path jar=Path.of(args[0]).toRealPath(); if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA)) throw new IOException("wrong client JAR SHA-256");
      byte[] input=System.in.readNBytes(MAX_BYTES+1); List<Row> rows=parse(input);
      // Force and retain the real registry's exact stdout bootstrap bytes.
      PrintStream actual=System.out; ByteArrayOutputStream capture=new ByteArrayOutputStream();
      System.setOut(new PrintStream(capture,true,StandardCharsets.UTF_8)); try { int ignored=uu.al.bn; } finally { System.setOut(actual); }
      byte[] boot=capture.toByteArray();
      if(!sha(boot).equals("0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e")) throw new IOException("unexpected original block-registry bootstrap bytes: sha="+sha(boot)+" base64="+Base64.getEncoder().encodeToString(boot));
      verifyNativeSurface();
      String text=run(rows,boot); byte[] output=text.getBytes(StandardCharsets.US_ASCII);
      String audit=audit(jar,input,output,boot,rows); Path auditPath=Path.of(args[1]); Files.writeString(auditPath,audit,StandardCharsets.UTF_8,StandardOpenOption.CREATE,StandardOpenOption.TRUNCATE_EXISTING);
      System.out.write(output); System.out.flush();
    } catch(Throwable failure) { System.err.println("LightOracle: "+failure); System.exit(2); }
  }
}
