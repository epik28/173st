import java.io.*;
import java.lang.reflect.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Synthetic water scenes rendered by the pinned Beta client without GL draw/flush. */
public final class FluidOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final String SUITE = "FLUID-001-C01\tids=8,9\tmetadata=0..15\tneighbors=east-source,west-flow,north-source,south-flow,diagonal-water,four-sources,water-above,flow-column,solid-east,solid-west,solid-north,solid-south";
  record Cell(int id, int meta) {}
  record Sample(String name, int id, int meta, Map<Long, Cell> cells) {}
  static final Map<Long, Cell> CELLS = new HashMap<>();
  static final Map<String, Integer> CALLS = new TreeMap<>();
  static final int CX=0, CY=64, CZ=0;
  static Field materialField, tessDataField, tessOffsetField, tessVertexCountField;

  static long key(int x,int y,int z) { return ((long)x << 42) ^ ((long)y << 21) ^ (z & 0x1fffffL); }
  static Cell cell(int x,int y,int z) { return CELLS.getOrDefault(key(x,y,z), new Cell(0,0)); }
  static String sha(byte[] bytes) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes)); }
  static String classSha(Class<?> type) throws Exception {
    try (InputStream in=type.getResourceAsStream("/"+type.getName().replace('.','/')+".class")) {
      if(in==null) throw new IOException("missing class bytes: "+type.getName()); return sha(in.readAllBytes());
    }
  }
  static void fromJar(Class<?> type,Path jar)throws Exception {
    Path source=Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
    if(!source.equals(jar))throw new IOException("class not loaded from pinned JAR: "+type.getName());
  }
  static int blockAt(int x,int y,int z) { return cell(x,y,z).id(); }
  static int metadataAt(int x,int y,int z) { return cell(x,y,z).meta(); }
  static void put(int x,int y,int z,int id,int meta) { if(id==0)CELLS.remove(key(x,y,z));else CELLS.put(key(x,y,z),new Cell(id,meta)); }
  static void floor() { for(int x=-1;x<=1;x++)for(int z=-1;z<=1;z++)put(x,CY-1,z,1,0); }
  static Sample sample(String name,int id,int meta,java.util.function.Consumer<Void> setup) {
    CELLS.clear(); floor(); setup.accept(null); return new Sample(name,id,meta,new HashMap<>(CELLS));
  }
  static List<Sample> samples() {
    List<Sample> result=new ArrayList<>();
    for(int id:new int[]{8,9})for(int meta=0;meta<16;meta++) {
      final int blockId=id, level=meta;
      result.add(sample("level-"+blockId+"-"+level,blockId,level,v->put(CX,CY,CZ,blockId,level)));
    }
    result.add(sample("east-source",9,0,v->{put(0,CY,0,9,0);put(1,CY,0,9,0);}));
    result.add(sample("west-flow",9,0,v->{put(0,CY,0,9,0);put(-1,CY,0,8,7);}));
    result.add(sample("north-source",9,0,v->{put(0,CY,0,9,0);put(0,CY,-1,9,0);}));
    result.add(sample("south-flow",9,0,v->{put(0,CY,0,9,0);put(0,CY,1,8,12);}));
    result.add(sample("diagonal-water",9,0,v->{put(0,CY,0,9,0);put(1,CY,1,9,0);}));
    result.add(sample("four-sources",9,0,v->{put(0,CY,0,9,0);put(1,CY,0,9,0);put(-1,CY,0,9,0);put(0,CY,1,9,0);put(0,CY,-1,9,0);}));
    result.add(sample("water-above",9,0,v->{put(0,CY,0,9,0);put(0,CY+1,0,9,0);}));
    result.add(sample("flow-column",8,8,v->{put(0,CY,0,8,8);put(0,CY-1,0,8,8);}));
    result.add(sample("solid-east",9,0,v->{put(0,CY,0,9,0);put(1,CY,0,1,0);}));
    result.add(sample("solid-west",9,0,v->{put(0,CY,0,9,0);put(-1,CY,0,1,0);}));
    result.add(sample("solid-north",9,0,v->{put(0,CY,0,9,0);put(0,CY, -1,1,0);}));
    result.add(sample("solid-south",9,0,v->{put(0,CY,0,9,0);put(0,CY,1,1,0);}));
    return result;
  }
  static Object world() {
    return Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
      String signature=m.getName()+"/"+(a==null?0:a.length); CALLS.merge(signature,1,Integer::sum);
      int x=a!=null&&a.length>0&&a[0] instanceof Integer?(int)a[0]:0;
      int y=a!=null&&a.length>1&&a[1] instanceof Integer?(int)a[1]:0;
      int z=a!=null&&a.length>2&&a[2] instanceof Integer?(int)a[2]:0;
      int id=blockAt(x,y,z);
      return switch(m.getName()) {
        case "a" -> m.getReturnType()==int.class?id:1.0f;
        case "e" -> metadataAt(x,y,z);
        case "f" -> id==0?ln.a:materialField.get(uu.m[id]);
        case "g" -> uu.m[id]!=null&&uu.m[id].c();
        case "h" -> false;
        case "c" -> 1.0f;
        default -> {Class<?> r=m.getReturnType();if(r==boolean.class)yield false;if(r==int.class)yield 0;if(r==float.class)yield 1.0f;if(r==double.class)yield 1.0d;yield null;}
      };
    });
  }
  static void emit(PrintStream out,cv renderer,nw tess,int[] words,Sample s)throws Exception {
    CELLS.clear();CELLS.putAll(s.cells());
    tessOffsetField.setInt(tess,0);tessVertexCountField.setInt(tess,0);int start=0;
    boolean rendered=renderer.b(uu.m[s.id()],CX,CY,CZ);
    int end=(int)tessOffsetField.get(tess);int count=(end-start)/8;
    out.printf(Locale.ROOT,"case\t%s\t%d\t%d\t%d\t%d%n",s.name(),s.id(),s.meta(),rendered?1:0,count);
    for(int i=start;i<end;i+=8){out.print("vertex\t"+s.name());for(int j=0;j<8;j++)out.printf(Locale.ROOT,"\t%08x",words[i+j]);out.println();}
  }
  public static void main(String[] args) {
    PrintStream report=System.out;
    try {
      if(args.length!=1)throw new IllegalArgumentException("usage: FluidOracle CLIENT_JAR");
      Path jar=Path.of(args[0]).toRealPath();
      if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("wrong client JAR SHA-256");
      System.setOut(new PrintStream(OutputStream.nullOutputStream()));
      materialField=uu.class.getDeclaredField("bA");materialField.setAccessible(true);
      for(Class<?> type:List.of(uu.class,cv.class,nw.class))fromJar(type,jar);
      ByteArrayOutputStream bootstrap=new ByteArrayOutputStream();System.setOut(new PrintStream(bootstrap,true,StandardCharsets.UTF_8));
      Object proxy=world();cv renderer=new cv((xp)proxy);nw tess=nw.a;
      uu registry=uu.m[9]; // Force registry bootstrap while its output is captured.
      tessDataField=nw.class.getDeclaredField("g");tessDataField.setAccessible(true);
      tessOffsetField=nw.class.getDeclaredField("o");tessOffsetField.setAccessible(true);
      tessVertexCountField=nw.class.getDeclaredField("h");tessVertexCountField.setAccessible(true);
      int[] words=(int[])tessDataField.get(tess);
      tess.b();
      System.setOut(report);
      report.println("#FLUID_NATIVE\t1\tjar="+JAR_SHA+"\tuu="+classSha(uu.class)+"\tcv="+classSha(cv.class)+"\tnw="+classSha(nw.class));
      report.println("#bootstrap\tbytes="+bootstrap.size()+"\tsha256="+sha(bootstrap.toByteArray())+"\tbase64="+Base64.getEncoder().encodeToString(bootstrap.toByteArray()));
      report.println("#input\t"+SUITE+"\tsha256="+sha(SUITE.getBytes(StandardCharsets.US_ASCII)));
      report.println("#setup\tsynthetic xp proxy; direct cv.b(uu,x,y,z); raw nw.g words; no native World/Chunk; no GL draw/flush");
      List<Sample> samples=samples();report.println("#cases\t"+samples.size());
      for(Sample sample:samples)emit(report,renderer,tess,words,sample);
      report.println("#native_calls\tcv.b(uu,int,int,int)="+samples.size()+"\tgl_draw=0\tgl_flush=0\tworld_native_loads=0");
      for(var call:CALLS.entrySet())report.println("#world_call\t"+call.getKey()+"\t"+call.getValue());
      report.flush();
    } catch(Throwable error) { error.printStackTrace(System.err);System.exit(1); }
  }
}
