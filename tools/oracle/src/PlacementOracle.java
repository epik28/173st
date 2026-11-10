import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;

/** Native placement probe against disposable byte-array chunks only. */
public final class PlacementOracle {
  static final String JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class DummyPlayer extends gs {
    DummyPlayer(){super(null);} public void v(){} public void a(vr sound,int count){}
  }
  static Object alloc(Class<?> c)throws Exception{
    Field f=Class.forName("sun.misc.Unsafe").getDeclaredField("theUnsafe"); f.setAccessible(true);
    return ((sun.misc.Unsafe)f.get(null)).allocateInstance(c);
  }
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
  record Case(String id,int target,int x,int y,int z,int side,int occupied,int repeats){}
  static List<Case> cases(){List<Case> out=new ArrayList<>();
    for(int target:new int[]{1,31})for(int side=0;side<6;side++)out.add(new Case("target"+target+"_side"+side,target,0,10,0,side,0,1));
    out.add(new Case("repeat_top",1,0,10,0,1,0,2));
    out.add(new Case("occupied_top",1,0,10,0,1,2,1));
    out.add(new Case("chunk_edge_east",1,15,10,0,5,0,1));
    out.add(new Case("place_at_127",1,0,126,0,1,0,1));
    return out;
  }
  static fd world(Provider p)throws Exception{
    fd seed=(fd)alloc(fd.class); Field f=fd.class.getDeclaredField("w");f.setAccessible(true);f.set(seed,p);
    f=fd.class.getDeclaredField("t");f.setAccessible(true);f.set(seed,xa.a(0));
    f=fd.class.getDeclaredField("x");f.setAccessible(true);f.set(seed,new ei(0L,"memory"));
    return fd.class.getConstructor(fd.class,xa.class).newInstance(seed,xa.a(0));
  }
  static void put(fd w,int x,int y,int z,int id,int meta){lm c=w.b(x>>4,z>>4);if(!c.a(x&15,y,z&15,id,meta))throw new IllegalStateException("native setup rejected block");}
  static String state(Provider p,int x,int y,int z){lm c=p.chunks.get(Provider.key(x>>4,z>>4));return c==null?"0:0":c.a(x&15,y,z&15)+":"+c.b(x&15,y,z&15);}
  static String neighborhood(Provider p,int x,int y,int z){StringJoiner j=new StringJoiner(",");for(int dy=-1;dy<=1;dy++)for(int dz=-1;dz<=1;dz++)for(int dx=-1;dx<=1;dx++)j.add(dx+":"+dy+":"+dz+"="+state(p,x+dx,y+dy,z+dz));return j.toString();}
  static int[] adjacent(Case c){return switch(c.side){case 0->new int[]{c.x,c.y-1,c.z};case 1->new int[]{c.x,c.y+1,c.z};case 2->new int[]{c.x,c.y,c.z-1};case 3->new int[]{c.x,c.y,c.z+1};case 4->new int[]{c.x-1,c.y,c.z};case 5->new int[]{c.x+1,c.y,c.z};default->throw new IllegalArgumentException();};}
  static String row(Case c)throws Exception{
    Provider p=new Provider();fd w=world(p);put(w,c.x,c.y,c.z,c.target,0);int[] q=adjacent(c);if(c.occupied!=0)put(w,q[0],q[1],q[2],c.occupied,0);
    DummyPlayer player=(DummyPlayer)alloc(DummyPlayer.class); ob controller=(ob)alloc(ob.class);iz stack=new iz(uu.m[1],4);
    double sx=c.x+.5,sy=c.y+.5,sz=c.z+.5,ex=sx,ey=sy,ez=sz;
    switch(c.side){case 0->sy=c.y-2;case 1->sy=c.y+2;case 2->sz=c.z-2;case 3->sz=c.z+2;case 4->sx=c.x-2;case 5->sx=c.x+2;}
    vf hit=w.a(bt.a(sx,sy,sz),bt.a(ex,ey,ez),false,false);
    String hitKind=hit==null?"miss":hit.a==jg.a?"block":"other",hitSide=hit==null?"-":Integer.toString(hit.e);
    String before=neighborhood(p,c.x,c.y,c.z), hitBefore=state(p,q[0],q[1],q[2]);
    int heightBefore=w.e(c.x,c.z), skyBefore=w.a(eb.a,c.x,c.y,c.z), blockBefore=w.a(eb.b,c.x,c.y,c.z);
    List<String> actions=new ArrayList<>();int countBefore=stack.a,damageBefore=stack.b;
    for(int i=0;i<c.repeats;i++)actions.add(Boolean.toString(controller.a(player,w,stack,c.x,c.y,c.z,c.side)));
    String after=neighborhood(p,c.x,c.y,c.z), placedAfter=state(p,q[0],q[1],q[2]);
    int heightAfter=w.e(c.x,c.z), skyAfter=w.a(eb.a,c.x,c.y,c.z), blockAfter=w.a(eb.b,c.x,c.y,c.z);
    return String.join("\t",c.id,Integer.toString(c.target),"0",Integer.toString(c.x),Integer.toString(c.y),Integer.toString(c.z),Integer.toString(c.side),hitKind,hitSide,
      String.join(",",actions),hitBefore,placedAfter,Integer.toString(countBefore),Integer.toString(stack.a),Integer.toString(damageBefore),Integer.toString(stack.b),
      Integer.toString(heightBefore),Integer.toString(heightAfter),Integer.toString(skyBefore),Integer.toString(skyAfter),Integer.toString(blockBefore),Integer.toString(blockAfter),before,after);
  }
  static byte[] capture(Path jar)throws Exception{
    if(!sha(Files.readAllBytes(jar)).equals(JAR_SHA))throw new IOException("pinned client JAR mismatch");
    PrintStream report=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    Class.forName("uu");Class.forName("gm");
    ByteArrayOutputStream bytes=new ByteArrayOutputStream();PrintStream out=new PrintStream(bytes,true,"US-ASCII");
    out.println("#BLOCK_PLACE_NATIVE\t1\t"+JAR_SHA);
    out.println("#id\ttarget_id\ttarget_meta\tx\ty\tz\tside\traycast_kind\traycast_side\tactions\tadjacent_before\tadjacent_after\tstack_before\tstack_after\tdamage_before\tdamage_after\theight_before\theight_after\tsky_before\tsky_after\tblocklight_before\tblocklight_after\tneighborhood_before\tneighborhood_after");
    for(Case c:cases())out.println(row(c));out.flush();System.setOut(report);return bytes.toByteArray();
  }
  static String sha(byte[] data)throws Exception{return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(data));}
  static String classSha(Class<?> c)throws Exception{try(InputStream in=c.getResourceAsStream("/"+c.getName().replace('.','/')+".class")){return sha(in.readAllBytes());}}
  static Map<String,String> classHashes(Path jar)throws Exception{
    Map<String,String> classes=new LinkedHashMap<>();
    for(Class<?> c:new Class<?>[]{fd.class,lm.class,uu.class,gm.class,ck.class,iz.class,ob.class,gs.class}){
      Path source=Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
      if(!source.equals(jar))throw new IOException("native class not loaded from pinned JAR: "+c.getName());
      classes.put(c.getName(),classSha(c));
    }
    return classes;
  }
  public static void main(String[] args)throws Exception{
    if(args.length!=1)throw new IllegalArgumentException("PlacementOracle CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();
    byte[] first=capture(jar),second=capture(jar);if(!Arrays.equals(first,second))throw new AssertionError("native action capture is not repeatable");
    System.out.write(first);
    Map<String,String> classes=classHashes(jar);
    System.err.println("{\"jar_sha256\":\""+JAR_SHA+"\",\"classes\":"+json(classes)+",\"native_cases\":"+cases().size()+",\"captures\":2,\"repeat_identical\":true,\"world_constructors\":0,\"save_access\":false,\"selected_item_id\":1,\"selected_block_id\":1,\"controller\":\"native ob.a(gs,fd,iz,x,y,z,side); dummy gs overrides sound only\",\"notes\":\"light and height values are recorded from the isolated minimally initialized world; they do not establish source-world lighting parity\"}");
  }
  static String json(Map<String,String> m){StringJoiner j=new StringJoiner(",","{","}");m.forEach((k,v)->j.add("\""+k+"\":\""+v+"\""));return j.toString();}
}
