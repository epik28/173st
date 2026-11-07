import java.io.*;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.zip.GZIPInputStream;

/** Read-only Beta world-data oracle: native ij/nu/ei/qj, without World/chunk factories. */
public final class WorldOracle {
  static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final long LIMIT = 64L * 1024 * 1024;
  static int nbtParses, eiConstructors, regionReads, chunkLoads;
  static String hex(byte[] b) { StringBuilder s=new StringBuilder(); for(byte x:b)s.append(String.format(Locale.ROOT,"%02x",x&255)); return s.toString(); }
  static String sha(byte[] b) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b)); }
  static String json(Object o) {
    if(o==null)return "null"; if(o instanceof Number||o instanceof Boolean)return o.toString();
    if(o instanceof Map<?,?> m){StringJoiner j=new StringJoiner(",","{","}");for(var e:m.entrySet())j.add(json(e.getKey().toString())+":"+json(e.getValue()));return j.toString();}
    if(o instanceof Collection<?> c){StringJoiner j=new StringJoiner(",","[","]");for(Object x:c)j.add(json(x));return j.toString();}
    String s=o.toString();StringBuilder b=new StringBuilder("\"");for(char c:s.toCharArray()){switch(c){case '"':b.append("\\\"");break;case '\\':b.append("\\\\");break;case '\n':b.append("\\n");break;case '\r':b.append("\\r");break;case '\t':b.append("\\t");break;default:if(c<32)b.append(String.format(Locale.ROOT,"\\u%04x",(int)c));else b.append(c);}}return b.append('"').toString();
  }
  static String classSha(Class<?> c) throws Exception { try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){if(in==null)throw new IOException("missing class bytes: "+c);return sha(in.readAllBytes());} }
  static byte[] bounded(InputStream in) throws IOException {
    ByteArrayOutputStream out=new ByteArrayOutputStream(); byte[] buf=new byte[8192]; int n;
    while((n=in.read(buf))!=-1){if((long)out.size()+n>LIMIT)throw new IOException("decompressed NBT limit");out.write(buf,0,n);} return out.toByteArray();
  }
  static nu parse(byte[] raw) throws Exception {
    DataInputStream in=new DataInputStream(new ByteArrayInputStream(raw)); ij tag=ij.b(in); nbtParses++;
    if(!(tag instanceof nu)||in.available()!=0)throw new IOException("NBT root must be one complete compound"); return (nu)tag;
  }
  static Map<String,ij> map(nu n) throws Exception { Field f=nu.class.getDeclaredField("a");f.setAccessible(true);return (Map<String,ij>)f.get(n); }
  static int tagCount(nu n) throws Exception { return map(n).size(); }
  static String unknown(nu n, Set<String> known) throws Exception {
    ArrayList<String> names=new ArrayList<>();for(String k:map(n).keySet())if(!known.contains(k))names.add(k.isEmpty()?"~":hex(k.getBytes(StandardCharsets.UTF_8)));
    Collections.sort(names);return names.isEmpty()?"-":String.join(",",names);
  }
  static Set<String> set(String... s){return Set.of(s);}
  static final Set<String> DATA=set("RandomSeed","SpawnX","SpawnY","SpawnZ","Time","LastPlayed","SizeOnDisk","LevelName","version","rainTime","raining","thunderTime","thundering","Player");
  static final Set<String> LEVEL=set("xPos","zPos","Blocks","Data","BlockLight","SkyLight","HeightMap","LastUpdate","TerrainPopulated","Entities","TileEntities");
  static void retain(Path dir,String id,byte[] bytes)throws IOException{Files.write(dir.resolve(id+".nbt"),bytes,StandardOpenOption.CREATE_NEW);}
  static String level(String id,byte[] raw,Path retained)throws Exception {
    nu root=parse(raw); if(!root.b("Data"))throw new IOException("missing Data");nu d=root.k("Data");ei e=new ei(d);eiConstructors++;
    nu p=e.h();int dimension=p==null?0:p.e("Dimension");
    String name=e.j().isEmpty()?"-":hex(e.j().getBytes(StandardCharsets.UTF_8));
    retain(retained,id,raw);
    return String.join("\t",id,"level",Long.toString(e.b()),Integer.toString(e.c()),Integer.toString(e.d()),Integer.toString(e.e()),Long.toString(e.f()),Long.toString(e.l()),Long.toString(e.g()),Integer.toString(dimension),name,Integer.toString(e.k()),Integer.toString(e.p()),e.o()?"1":"0",Integer.toString(e.n()),e.m()?"1":"0",p==null?"0":"1",Integer.toString(p==null?0:tagCount(p)),unknown(d,DATA),Integer.toString(raw.length),sha(raw));
  }
  static long longRequired(nu n,String name)throws Exception{return n.f(name);}
  static byte[] array(nu n,String name)throws Exception{return n.j(name);}
  static String chunk(String id,int cx,int cz,byte[] raw,Path retained)throws Exception {
    nu root=parse(raw);if(!root.b("Level"))throw new IOException("missing Level");nu l=root.k("Level");
    int x=l.e("xPos"),z=l.e("zPos");if(x!=cx||z!=cz)throw new IOException("coordinate mismatch");
    byte[] blocks=array(l,"Blocks"),data=array(l,"Data"),bl=array(l,"BlockLight"),sl=array(l,"SkyLight"),hm=array(l,"HeightMap");
    if(blocks.length!=32768||data.length!=16384||bl.length!=16384||sl.length!=16384||hm.length!=256)throw new IOException("invalid native array lengths");
    boolean terrain=l.m("TerrainPopulated");long last=l.f("LastUpdate");
    sp entities=l.l("Entities"),tiles=l.l("TileEntities");int ec=entities==null?0:entities.c(),tc=tiles==null?0:tiles.c();
    retain(retained,id,raw);
    return String.join("\t",id,"chunk","present",Integer.toString(cx),Integer.toString(cz),Long.toString(last),terrain?"1":"0",sha(blocks),sha(data),sha(bl),sha(sl),sha(hm),Integer.toString(ec),Integer.toString(tc),unknown(l,LEVEL),Integer.toString(raw.length),sha(raw));
  }
  static Map<String,String> codeSources(Class<?>... classes)throws Exception{Map<String,String> r=new TreeMap<>();for(Class<?> c:classes)r.put(c.getName(),Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().toString());return r;}
  static byte[] encoded(ij tag)throws Exception{ByteArrayOutputStream b=new ByteArrayOutputStream();ij.a(tag,new DataOutputStream(b));return b.toByteArray();}
  static void selfTest(Map<String,Object> audit,Path retained)throws Exception {
    List<String> cases=new ArrayList<>();
    nu missing=new nu();ei e=new ei(missing);eiConstructors++;
    if(e.b()!=0||e.c()!=0||e.j().length()!=0||e.o()||e.h()!=null)throw new IOException("native missing-default selftest");cases.add("missing-scalars-and-player");
    nu byteBool=new nu();byteBool.a("raining",(byte)0x80);byteBool.a("thundering",(byte)-1);ei b=new ei(byteBool);eiConstructors++;if(!b.o()||!b.m())throw new IOException("native nonzero boolean byte selftest");cases.add("nonzero-and-negative-boolean-byte");
    nu explicit=new nu();explicit.a("Player",new nu());ei ep=new ei(explicit);eiConstructors++;if(ep.h()==null||ep.i()!=0||tagCount(ep.h())!=0)throw new IOException("native empty Player selftest");cases.add("empty-player-compound");
    nu bounds=new nu();bounds.a("RandomSeed",Long.MIN_VALUE);bounds.a("SpawnX",Integer.MIN_VALUE);bounds.a("SpawnY",Integer.MAX_VALUE);bounds.a("SpawnZ",Integer.MIN_VALUE);bounds.a("Time",Long.MAX_VALUE);bounds.a("LastPlayed",Long.MIN_VALUE);bounds.a("SizeOnDisk",Long.MAX_VALUE);bounds.a("LevelName","");bounds.a("version",Integer.MAX_VALUE);bounds.a("rainTime",Integer.MIN_VALUE);bounds.a("thunderTime",Integer.MAX_VALUE);ei be=new ei(bounds);eiConstructors++;if(be.b()!=Long.MIN_VALUE||be.c()!=Integer.MIN_VALUE||be.d()!=Integer.MAX_VALUE||be.e()!=Integer.MIN_VALUE||be.f()!=Long.MAX_VALUE||be.l()!=Long.MIN_VALUE||be.g()!=Long.MAX_VALUE||!be.j().isEmpty())throw new IOException("native numeric/string boundaries");cases.add("i32-i64-boundaries-and-empty-name");
    boolean wrongSeed=false;nu bad=new nu();bad.a("RandomSeed",7);eiConstructors++;try{new ei(bad);}catch(ClassCastException expected){wrongSeed=true;}
    boolean wrongByte=false;nu badByte=new nu();badByte.a("raining",7);eiConstructors++;try{new ei(badByte);}catch(ClassCastException expected){wrongByte=true;}
    boolean wrongPlayer=false;nu badPlayer=new nu();badPlayer.a("Player",7);eiConstructors++;try{new ei(badPlayer);}catch(ClassCastException expected){wrongPlayer=true;}
    if(!wrongSeed||!wrongByte||!wrongPlayer)throw new IOException("native wrong-type selftest");cases.add("wrong-long-int-byte-compound-types-cast-fail");
    nu d=new nu();d.a("",1);d.a("mystery☃",-7);nu droot=new nu();droot.a("Data",d);String unk=unknown(parse(encoded(droot)).k("Data"),DATA);if(!unk.equals("6d797374657279e29883,~"))throw new IOException("unexpected unknown name encoding: "+unk);cases.add("unknown-empty-and-unicode-names");
    nu level=new nu();level.a("xPos",-1);level.a("zPos",2);byte[] blocks=new byte[32768],nibbles=new byte[16384],height=new byte[256];blocks[0]=(byte)0x80;blocks[1]=(byte)0xff;nibbles[0]=(byte)0x80;nibbles[1]=(byte)0xff;height[0]=(byte)0xff;
    level.a("Blocks",blocks);level.a("Data",nibbles);level.a("BlockLight",nibbles);level.a("SkyLight",nibbles);level.a("HeightMap",height);level.a("TerrainPopulated",(byte)0x80);
    nu entity=new nu();entity.a("id","future_entity");sp entities=new sp();entities.a(entity);level.a("Entities",entities);
    nu tile=new nu();tile.a("id","unknown_tile");sp tiles=new sp();tiles.a(tile);level.a("TileEntities",tiles);level.a("",1);level.a("mystery☃",2);
    nu chunkRoot=new nu();chunkRoot.a("Level",level);byte[] raw=encoded(chunkRoot);
    String row=chunk("generated-chunk",-1,2,raw,retained);String[] fields=row.split("\t",-1);if(!fields[7].equals(sha(blocks))||!fields[8].equals(sha(nibbles))||!fields[5].equals("0")||!fields[6].equals("1")||!fields[12].equals("1")||!fields[13].equals("1")||!fields[14].contains("6d797374657279e29883"))throw new IOException("generated native chunk extraction");
    cases.add("chunk-signed-bytes-lists-unknown-names-preserved");
    sp emptyBytes=new sp();Field listType=sp.class.getDeclaredField("b");listType.setAccessible(true);listType.setByte(emptyBytes,(byte)1);level.a("Entities",emptyBytes);map(level).remove("TileEntities");
    String emptyRow=chunk("generated-empty-byte-list",-1,2,encoded(chunkRoot),retained);if(!emptyRow.split("\t",-1)[12].equals("0")||!emptyRow.split("\t",-1)[13].equals("0"))throw new IOException("empty byte list was not preserved");cases.add("empty-entities-byte-list-and-absent-tile-list");
    boolean wrongArrayType=false;nu wrongChunkRoot=(nu)parse(raw);wrongChunkRoot.k("Level").a("Blocks",7);try{chunk("invalid-generated-type",-1,2,encoded(wrongChunkRoot),retained);}catch(ClassCastException expected){wrongArrayType=true;}
    boolean wrongLength=false;nu shortArrayRoot=(nu)parse(raw);shortArrayRoot.k("Level").a("Blocks",new byte[3]);try{chunk("invalid-generated-length",-1,2,encoded(shortArrayRoot),retained);}catch(IOException expected){wrongLength=true;}
    boolean mismatch=false;try{chunk("invalid-generated-coordinate",-2,2,raw,retained);}catch(IOException expected){mismatch=true;}
    boolean missingLevel=false;try{chunk("invalid-generated-level",-1,2,encoded(new nu()),retained);}catch(IOException expected){missingLevel=true;}
    if(!wrongArrayType||!wrongLength||!mismatch||!missingLevel)throw new IOException("generated strict chunk errors not observed");cases.add("wrong-array-type-length-coordinate-and-missing-Level-errors");
    audit.put("generated_semantics",Map.of("cases",cases,"status","PASS","wrong_type_class_cast","PASS","unknown_entity_and_tile_factories","not_called","native_chunk_loads",0));
  }
  public static void main(String[] args)throws Exception{
    if(args.length==4&&args[0].equals("--self-test")){
      Path jar=Path.of(args[1]).toRealPath(),retained=Path.of(args[2]).toAbsolutePath().normalize(),auditPath=Path.of(args[3]).toAbsolutePath().normalize();
      if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("wrong JAR hash");
      Files.createDirectory(retained);Map<String,Object> generated=new LinkedHashMap<>();
      generated.put("jar_sha256",JAR_SHA);generated.put("classes",Map.of("ij",classSha(ij.class),"nu",classSha(nu.class),"ei",classSha(ei.class),"qj",classSha(qj.class)));
      generated.put("code_sources",codeSources(ij.class,nu.class,ei.class,qj.class));selfTest(generated,retained);
      generated.put("native_calls",Map.of("ij_b_DataInput",nbtParses,"ei_constructor",eiConstructors,"qj_a_int_int",regionReads,"native_chunk_loads",chunkLoads));
      Map<String,String> rawHashes=new TreeMap<>();try(DirectoryStream<Path> ds=Files.newDirectoryStream(retained,"*.nbt")){for(Path p:ds)rawHashes.put(p.getFileName().toString(),sha(Files.readAllBytes(p)));}
      generated.put("retained_files",rawHashes);Files.writeString(auditPath,json(generated)+"\n",StandardOpenOption.CREATE_NEW);return;
    }
    if(args.length!=4)throw new IllegalArgumentException("WorldOracle JAR INPUT_COPY_DIR RETAINED_DIR AUDIT_JSON");
    Path jar=Path.of(args[0]).toRealPath(),input=Path.of(args[1]).toRealPath(),retained=Path.of(args[2]).toAbsolutePath().normalize(),auditPath=Path.of(args[3]).toAbsolutePath().normalize();
    if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("wrong JAR hash");
    for(Class<?> c:new Class<?>[]{ij.class,nu.class,ei.class,qj.class})if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("native class not loaded from configured JAR: "+c);
    if(Files.isSymbolicLink(input.resolve(".world-input-copy"))||Files.isSymbolicLink(input.resolve(".region-input-copy"))||!Files.readString(input.resolve(".world-input-copy")).equals("WORLD-LOAD-001 copied inputs\n")||!Files.readString(input.resolve(".region-input-copy")).equals("REGION-001 copied inputs\n"))throw new IOException("copy markers missing/unsafe");
    if(retained.startsWith(input)||input.startsWith(retained)||auditPath.startsWith(input)||input.startsWith(auditPath)||retained.startsWith(auditPath)||auditPath.startsWith(retained))throw new IOException("output/input overlap");
    Files.createDirectory(retained);
    Map<String,String> inputHashes=new TreeMap<>();try(DirectoryStream<Path> ds=Files.newDirectoryStream(input)){for(Path p:ds)if(!Files.isDirectory(p)&&!p.getFileName().toString().startsWith(".")){if(Files.isSymbolicLink(p)||!p.toRealPath().getParent().equals(input))throw new IOException("unsafe input path");inputHashes.put(p.getFileName().toString(),sha(Files.readAllBytes(p)));}}
    if(!inputHashes.keySet().containsAll(List.of("level.dat","r.-1.-1.mcr","r.-1.0.mcr","r.0.-1.mcr","r.0.0.mcr")))throw new IOException("five world inputs required");
    Map<String,Object> audit=new LinkedHashMap<>();audit.put("jar_sha256",JAR_SHA);audit.put("classes",Map.of("ij",classSha(ij.class),"nu",classSha(nu.class),"ei",classSha(ei.class),"qj",classSha(qj.class)));audit.put("code_sources",codeSources(ij.class,nu.class,ei.class,qj.class));audit.put("native_methods",Map.of("nbt_parse","ij.b(DataInput)","metadata_constructor","ei(nu)","region_read","qj.a(int,int)","native_chunk_loads",0));
    int levelCount=0,present=0,absent=0;Map<Path,qj> regions=new LinkedHashMap<>();
    try(BufferedReader r=new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8))){String line;Set<String> ids=new HashSet<>();while((line=r.readLine())!=null){if(line.isEmpty()||line.startsWith("#"))continue;String[] f=line.split("\t",-1);if(f.length!=5||!f[0].matches("[A-Za-z0-9_-]+")||!ids.add(f[0])||ids.size()>100000)throw new IOException("invalid/duplicate ID or case limit");
      if(f[1].equals("level")){if(!f[2].equals("level.dat")||!f[3].equals("-")||!f[4].equals("-"))throw new IOException("invalid level row");Path p=input.resolve("level.dat");if(Files.size(p)>16*1024*1024)throw new IOException("compressed level limit");byte[] packed=Files.readAllBytes(p);byte[] raw;try(InputStream gz=new GZIPInputStream(new ByteArrayInputStream(packed))){raw=bounded(gz);}System.out.println(level(f[0],raw,retained));levelCount++;}
      else if(f[1].equals("chunk")){if(!f[2].startsWith("r.")||!f[2].endsWith(".mcr")||!f[3].matches("-?(0|[1-9][0-9]*)")||!f[4].matches("-?(0|[1-9][0-9]*)"))throw new IOException("invalid chunk row");int x=Integer.parseInt(f[3]),z=Integer.parseInt(f[4]);String expected="r."+Math.floorDiv(x,32)+"."+Math.floorDiv(z,32)+".mcr";if(!f[2].equals(expected))throw new IOException("region-coordinate mismatch");Path p=input.resolve(f[2]);if(Files.isSymbolicLink(p)||!p.toRealPath().getParent().equals(input))throw new IOException("unsafe region path");qj q=regions.get(p);if(q==null){q=new qj(p.toFile());regions.put(p,q);}int lx=Math.floorMod(x,32),lz=Math.floorMod(z,32);regionReads++;try(DataInputStream in=q.a(lx,lz)){if(in==null){System.out.println(String.join("\t",f[0],"chunk","absent",f[3],f[4],"-","-","-","-","-","-","-","-","-","-","-","-"));absent++;}else{byte[] raw=bounded(in);System.out.println(chunk(f[0],x,z,raw,retained));present++;}}}
      else throw new IOException("unknown case kind");}}
    finally { for(qj q:regions.values())q.b(); }
    audit.put("native_calls",Map.of("ij_b_DataInput",nbtParses,"ei_constructor",eiConstructors,"qj_a_int_int",regionReads,"native_chunk_loads",chunkLoads));audit.put("live_rows",levelCount+present+absent);audit.put("level_rows",levelCount);audit.put("chunk_present",present);audit.put("chunk_absent",absent);audit.put("input_sha256",inputHashes);audit.put("retained_files",new TreeMap<String,String>());
    Map<String,String> retainedHashes=(Map<String,String>)audit.get("retained_files");try(DirectoryStream<Path> ds=Files.newDirectoryStream(retained,"*.nbt")){for(Path p:ds)retainedHashes.put(p.getFileName().toString(),sha(Files.readAllBytes(p)));}
    audit.put("class_names",List.of("ij","nu","ei","qj"));Files.writeString(auditPath,json(audit)+"\n",StandardOpenOption.CREATE_NEW);
  }
}
