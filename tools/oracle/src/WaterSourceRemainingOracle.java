import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Bounded native water update probe; creates only in-memory chunks via a synthetic provider. */
public final class WaterSourceRemainingOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final int CX=0,CY=64,CZ=0;
  static final Map<Long,lm> CHUNKS=new HashMap<>();
  static final List<String> EVENTS=new ArrayList<>();
  static long key(int x,int z){return ((long)x<<32)^(z&0xffffffffL);}
  static final class Provider implements wt {
    public ei c(){return null;} public void b(){}
    public bf a(xa dimension){return new bf(){
      public lm a(fd world,int x,int z){return CHUNKS.computeIfAbsent(key(x,z),k->new lm(world,new byte[32768],x,z));}
      public void a(fd world,lm chunk){CHUNKS.put(key(chunk.j,chunk.k),chunk);}
      public void b(fd world,lm chunk){CHUNKS.put(key(chunk.j,chunk.k),chunk);}
      public void a(){} public void b(){}
    };}
    public void a(ei info,List list){} public void a(ei info){} public File a(String name){return null;}
  }
  static final class TrackedWorld extends fd {
    TrackedWorld(fd seed){super(seed,xa.a(0));}
    @Override public boolean a(int x,int y,int z,int id,int meta){
      int old=a(x,y,z), oldMeta=e(x,y,z);
      boolean changed=super.a(x,y,z,id,meta);
      if(changed)EVENTS.add("edit\t"+x+"\t"+y+"\t"+z+"\t"+old+"\t"+oldMeta+"\t"+id+"\t"+meta);
      return changed;
    }
    @Override public void c(int x,int y,int z,int id,int delay){EVENTS.add("schedule\t"+x+"\t"+y+"\t"+z+"\t"+id+"\t"+delay);super.c(x,y,z,id,delay);}
    @Override public void i(int x,int y,int z,int id){EVENTS.add("neighbor\t"+x+"\t"+y+"\t"+z+"\t"+id);super.i(x,y,z,id);}
  }
  static Object alloc(Class<?> c)throws Exception{Field f=Class.forName("sun.misc.Unsafe").getDeclaredField("theUnsafe");f.setAccessible(true);return ((sun.misc.Unsafe)f.get(null)).allocateInstance(c);}
  static TrackedWorld world()throws Exception{
    CHUNKS.clear();EVENTS.clear();fd seed=(fd)alloc(fd.class);
    Field provider=fd.class.getDeclaredField("w");provider.setAccessible(true);provider.set(seed,new Provider());
    Field dimension=fd.class.getDeclaredField("t");dimension.setAccessible(true);dimension.set(seed,xa.a(0));
    Field info=fd.class.getDeclaredField("x");info.setAccessible(true);info.set(seed,new ei(0L,"synthetic-memory"));
    TrackedWorld world=new TrackedWorld(seed);
    for(int z=-1;z<=1;z++)for(int x=-1;x<=1;x++)world.b(x,z);
    return world;
  }
  static void put(fd w,int x,int y,int z,int id,int meta){if(!w.a(x,y,z,id,meta))throw new IllegalStateException("setup write rejected");}
  static String state(fd w,int x,int y,int z){return w.a(x,y,z)+":"+w.e(x,y,z);}
  static String region(fd w,int radius){StringBuilder b=new StringBuilder();for(int y=CY+1;y>=CY-2;y--)for(int z=-radius;z<=radius;z++)for(int x=-radius;x<=radius;x++)b.append(x).append(',').append(y).append(',').append(z).append('=').append(state(w,x,y,z)).append(';');return b.toString();}
  static byte[] sha(byte[] data)throws Exception{return MessageDigest.getInstance("SHA-256").digest(data);}
  static String hex(byte[] data)throws Exception{return HexFormat.of().formatHex(sha(data));}
  static String classSha(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){return hex(in.readAllBytes());}}
  static void fromJar(Class<?> c,Path jar)throws Exception{if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("class not loaded from pinned jar: "+c.getName());}
  record Scene(String name,int id,int metadata,long rngSeed,int[][] setup,int ticks){}
  record ScheduleScene(String name,long rngSeed,int[][] setup,int operation){}
  static List<Scene> scenes(){return List.of(
    new Scene("west-south-sources",8,1,17804,new int[][]{{0,64,0,8,1},{-1,64,0,9,0},{0,64,1,9,0},{0,63,0,1,0},{1,64,0,1,0},{0,64,-1,1,0}},2),
    new Scene("east-north-sources",8,1,17805,new int[][]{{0,64,0,8,1},{1,64,0,9,0},{0,64,-1,9,0},{0,63,0,1,0},{-1,64,0,1,0},{0,64,1,1,0}},2)
  );}
  static List<Scene> sourceScenes(){return List.of(
    new Scene("two-neighbors-supported-meta1",8,1,17311,new int[][]{{0,64,0,8,1},{-1,64,0,9,0},{1,64,0,9,0},{0,63,0,1,0},{0,64,-1,1,0},{0,64,1,1,0}},2),
    new Scene("one-neighbor-supported-meta1",8,1,17312,new int[][]{{0,64,0,8,1},{-1,64,0,9,0},{1,64,0,1,0},{0,63,0,1,0},{0,64,-1,1,0},{0,64,1,1,0}},1),
    new Scene("two-neighbors-unsupported-meta1",8,1,17313,new int[][]{{0,64,0,8,1},{-1,64,0,9,0},{1,64,0,9,0},{0,64,-1,1,0},{0,64,1,1,0}},1),
    new Scene("two-neighbors-source-below-meta1",8,1,17314,new int[][]{{0,64,0,8,1},{-1,64,0,9,0},{1,64,0,9,0},{0,63,0,9,0},{0,64,-1,1,0},{0,64,1,1,0}},1)
  );}
  static List<ScheduleScene> scheduleScenes(){return List.of(
    new ScheduleScene("initial-flow-placement",17321,new int[][]{},0),
    new ScheduleScene("place-adjacent-solid-neighbor",17322,new int[][]{{0,63,0,1,0},{0,64,0,9,0}},1),
    new ScheduleScene("remove-still-water-support",17323,new int[][]{{0,63,0,1,0},{0,64,0,9,0}},2)
  );}
  static String queue(fd world)throws Exception{
    Field f=fd.class.getDeclaredField("E");f.setAccessible(true);TreeSet<?> queue=(TreeSet<?>)f.get(world);StringJoiner rows=new StringJoiner(",");
    for(Object event:queue){Class<?> c=event.getClass();StringJoiner j=new StringJoiner(":");for(String name:new String[]{"a","b","c","d","e"}){Field p=c.getDeclaredField(name);p.setAccessible(true);j.add(String.valueOf(p.get(event)));}rows.add(j.toString());}
    return rows.toString();
  }
  static byte[] capture(Path jar,boolean sourcesOnly)throws Exception{
    if(!hex(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("wrong pinned client JAR SHA");
    PrintStream report=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    Class.forName("uu");Class.forName("om");Class.forName("nx");
    String omHash=classSha(om.class),nxHash=classSha(nx.class),rpHash=classSha(rp.class),fdHash=classSha(fd.class),qyHash=classSha(qy.class);
    for(Class<?> c:new Class<?>[]{uu.class,om.class,nx.class,rp.class,fd.class,qy.class,lm.class})fromJar(c,jar);
    ByteArrayOutputStream output=new ByteArrayOutputStream();PrintStream out=new PrintStream(output,true,"US-ASCII");
    out.println("#WATER_TICK_NATIVE\t1\tjar="+JAR_SHA+"\tuu="+classSha(uu.class)+"\tom="+omHash+"\tnx="+nxHash+"\trp="+rpHash+"\tfd="+fdHash+"\tqy="+qyHash);
    out.println("#setup\tfd copy-constructor over Unsafe seed; custom wt/bf with byte-array lm chunks only; fixed origin (0,64,0); nine in-memory chunks (-1..1); no save/provider reads");
    out.println("#methods\tom.a(fd,int,int,int,Random); nx.a(fd,int,int,int,Random); fd.c(int,int,int,int,int); fd.a(boolean); fd.i(int,int,int,int)");
    out.println("#bounds\tstate snapshot x,z=-2..2 y=62..65; loaded chunks are synthetic and all loaded; no unloaded-neighbor result is claimed");
    out.println("#loaded_chunks\t-1,-1;-1,0;-1,1;0,-1;0,0;0,1;1,-1;1,0;1,1");
    Field randomField=fd.class.getDeclaredField("r");randomField.setAccessible(true);
    for(Scene s:(sourcesOnly?sourceScenes():scenes())){
      TrackedWorld w=world();randomField.set(w,new Random(s.rngSeed()));
      EVENTS.clear();for(int[] p:s.setup())put(w,p[0],p[1],p[2],p[3],p[4]);
      if(s.name().equals("still-source")){w.c(CX,CY,CZ,s.id(),0);EVENTS.add("probe_schedule\t"+CX+"\t"+CY+"\t"+CZ+"\t"+s.id()+"\t0");}
      out.println("case\t"+s.name()+"\tseed_id="+s.id()+"\tinput_meta="+s.metadata()+"\tticks="+s.ticks()+"\trng_seed="+s.rngSeed());
      for(int[] p:s.setup())out.println("input\t"+p[0]+"\t"+p[1]+"\t"+p[2]+"\t"+p[3]+"\t"+p[4]);
      out.println("setup_queue\t"+queue(w));for(String event:EVENTS)out.println("setup_"+event);
      for(int tick=0;tick<s.ticks();tick++){
        String before=queue(w);if(before.isEmpty())break;
        long due=Long.MAX_VALUE;for(String row:before.split(",")){String[] values=row.split(":");due=Math.min(due,Long.parseLong(values[4]));}
        w.x.a(due+1L);out.println("queue_before\t"+before+"\tworld_time="+w.x.f());EVENTS.clear();
        boolean pending=w.a(false);
        out.println("step\t"+tick+"\tpending="+(pending?1:0)+"\tqueue_after="+queue(w));
        out.println("state\t"+region(w,2));for(String event:EVENTS)out.println(event);
      }
      out.println("#case_end\t"+s.name());
    }
    out.flush();System.setOut(report);return output.toByteArray();
  }
  static byte[] captureSchedule(Path jar)throws Exception{
    if(!hex(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("wrong pinned client JAR SHA");
    PrintStream report=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    Class.forName("uu");Class.forName("om");Class.forName("nx");
    String omHash=classSha(om.class),nxHash=classSha(nx.class),rpHash=classSha(rp.class),fdHash=classSha(fd.class),qyHash=classSha(qy.class);
    for(Class<?> c:new Class<?>[]{uu.class,om.class,nx.class,rp.class,fd.class,qy.class,lm.class})fromJar(c,jar);
    ByteArrayOutputStream output=new ByteArrayOutputStream();PrintStream out=new PrintStream(output,true,"US-ASCII");
    out.println("#WATER_SCHEDULE_NATIVE\t1\tjar="+JAR_SHA+"\tuu="+classSha(uu.class)+"\tom="+omHash+"\tnx="+nxHash+"\trp="+rpHash+"\tfd="+fdHash+"\tqy="+qyHash);
    out.println("#setup\tfd Unsafe seed; custom wt/bf with nine byte-array lm chunks (-1..1); all synthetic chunks loaded; time starts at 0; no save/provider reads");
    out.println("#methods\tfd.a(int,int,int,int,int); fd.i(int,int,int,int); fd.c(int,int,int,int,int); om.b(fd,int,int,int,int); nx.b(fd,int,int,int,int)");
    out.println("#bounds\tstate snapshot x,z=-1..1 y=62..65; only explicit fd.i neighbor notifications are exercised");
    Field randomField=fd.class.getDeclaredField("r");randomField.setAccessible(true);
    for(ScheduleScene s:scheduleScenes()){
      TrackedWorld w=world();randomField.set(w,new Random(s.rngSeed()));EVENTS.clear();
      for(int[] p:s.setup())put(w,p[0],p[1],p[2],p[3],p[4]);
      out.println("case\t"+s.name()+"\trng_seed="+s.rngSeed()+"\toperation="+s.operation());
      for(int[] p:s.setup())out.println("input\t"+p[0]+"\t"+p[1]+"\t"+p[2]+"\t"+p[3]+"\t"+p[4]);
      out.println("setup_queue\t"+queue(w));for(String event:EVENTS)out.println("setup_"+event);
      EVENTS.clear();out.println("queue_before\t"+queue(w));
      if(s.operation()==0){out.println("action\tplace-flow\t0\t64\t0\t8\t0");w.a(0,64,0,8,0);}
      else if(s.operation()==1){out.println("action\tplace-solid-and-notify\t1\t64\t0\t1\t0");w.a(1,64,0,1,0);w.i(1,64,0,1);}
      else {out.println("action\tremove-support-and-notify\t0\t63\t0\t0\t0");w.a(0,63,0,0,0);w.i(0,63,0,0);}
      for(String event:EVENTS)out.println(event);
      out.println("queue_after\t"+queue(w));out.println("state\t"+region(w,1));out.println("#case_end\t"+s.name());
    }
    out.flush();System.setOut(report);return output.toByteArray();
  }
  public static void main(String[] args)throws Exception{boolean sources=args.length==2&&args[1].equals("--sources"),schedule=args.length==2&&args[1].equals("--schedule");if(args.length<1||args.length>2||(args.length==2&&!sources&&!schedule))throw new IllegalArgumentException("WaterTickOracle CLIENT_JAR [--sources|--schedule]");Path jar=Path.of(args[0]).toRealPath();byte[] a=schedule?captureSchedule(jar):capture(jar,sources),b=schedule?captureSchedule(jar):capture(jar,sources);if(!Arrays.equals(a,b))throw new AssertionError("native water captures differ");System.out.write(a);}
}
