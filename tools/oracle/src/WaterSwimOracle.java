import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.HashSet;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.jar.JarFile;
import sun.misc.Unsafe;

/** Bounded player travel through a sparse, synthetic Beta 1.7.3 water world. */
public final class WaterSwimOracle {
  private static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  private static final Pattern ID = Pattern.compile("[A-Za-z0-9_.:-]{1,96}");
  private static final Pattern HEX64 = Pattern.compile("[0-9a-f]{16}");
  private static final Pattern HEX32 = Pattern.compile("[0-9a-f]{8}");
  private static final String[] NATIVE_CLASSES = {"sn", "ls", "gs", "eq", "fd", "uu", "ln", "rp", "bt", "in"};
  private static final String[] FIELD_MAPPING = {
      "position_x=aM", "position_y=aN", "position_z=aO",
      "velocity_x=aP", "velocity_y=aQ", "velocity_z=aR",
      "bbox_min_x=aW.a", "bbox_min_y=aW.b", "bbox_min_z=aW.c",
      "bbox_max_x=aW.d", "bbox_max_y=aW.e", "bbox_max_z=aW.f",
      "y_offset=bf", "y_size=bo", "step_height=bp", "fall_distance=bk",
      "yaw_degrees=aS", "on_ground=aX", "collided_horizontally=aY",
      "collided_vertically=aZ", "collided=ba", "no_clip=bq", "in_web=bc",
      "sneaking=t()", "allow_slide=bd", "in_water=bx"};
  private static long travelCalls, jumpCalls, updateCalls, collisionQueries, waterBoxQueries, blockIdQueries, metadataQueries, nativeWaterQueries, nativeWaterContactQueries;
  private static int worldConstructors;
  private static Method nativePlayerTravel, nativeJump;
  private static Path jarPath;

  private static final class WaterCell {
    final int x,y,z,id,meta;
    WaterCell(int x,int y,int z,int id,int meta){this.x=x;this.y=y;this.z=z;this.id=id;this.meta=meta;}
  }
  private static final class State {
    double[] p=new double[3],v=new double[3],box=new double[6];
    float yOffset,ySize,stepHeight,fallDistance,yaw;
    int flags;
  }
  private static final class Frame {
    float strafe,forward,yaw;
    boolean jump,sneak;
  }
  private static final class Scenario {
    String id;
    State initial;
    final List<WaterCell> water=new ArrayList<>();
    final List<eq> colliders=new ArrayList<>();
    final List<Frame> frames=new ArrayList<>();
  }

  /** Allocated without fd constructors/chunk provider; every query is backed by fixture data. */
  public static final class SyntheticWorld extends fd {
    List<WaterCell> water;
    List<eq> colliders;
    SyntheticWorld(){super((wt)null,"unused",0L);worldConstructors++;}
    @Override public br u(){return new br(0,64,0);}
    @Override public int a(int x,int y,int z){blockIdQueries++;for(WaterCell c:water)if(c.x==x&&c.y==y&&c.z==z)return c.id;return 0;}
    @Override public int e(int x,int y,int z){metadataQueries++;for(WaterCell c:water)if(c.x==x&&c.y==y&&c.z==z)return c.meta;return 0;}
    @Override public boolean a(int x1,int y1,int z1,int x2,int y2,int z2){return true;}
    @Override public List a(sn entity,eq region){
      collisionQueries++;List<eq> hits=new ArrayList<>();
      for(eq collider:colliders)if(collider.a(region))hits.add(collider);
      return hits;
    }
    @Override public List b(sn entity,eq region){return java.util.Collections.emptyList();}
    @Override public boolean b(eq region){
      collisionQueries++;for(eq collider:colliders)if(collider.a(region))return true;return false;
    }
    @Override public boolean a(eq region,ln material,sn entity){
      waterBoxQueries++;
      return super.a(region,material,entity);
    }
    @Override public boolean a(eq region,ln material){waterBoxQueries++;return super.a(region,material);}
    @Override public boolean b(eq region,ln material){return false;}
    @Override public void a(sn entity,String sound,float volume,float pitch){}
    @Override public void a(String name,double x,double y,double z,double vx,double vy,double vz){}
  }

  /** The exact native gs player movement wrapper, with only fixture counters added. */
  public static final class SyntheticPlayer extends gs {
    long waterQueries,waterContactQueries;
    SyntheticPlayer(fd world){super(world);}
    @Override public void v(){}
    @Override protected void f_(){}
    void controls(boolean jump, float strafe, float forward){az=jump; aw=strafe; ax=forward;}
    @Override public boolean a(ln material){if(material==ln.g){waterQueries++;nativeWaterQueries++;}return super.a(material);}
    @Override public boolean k_(){waterContactQueries++;nativeWaterContactQueries++;return super.k_();}
  }

  private static String sha(byte[] bytes)throws Exception{return java.util.HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));}
  private static String doubleBits(double x){return String.format(Locale.ROOT,"%016x",Double.doubleToRawLongBits(x));}
  private static String floatBits(float x){return String.format(Locale.ROOT,"%08x",Float.floatToRawIntBits(x));}
  private static double parseDouble(String raw){
    if(!HEX64.matcher(raw).matches())throw new IllegalArgumentException("expected lowercase raw f64 hex");
    double v=Double.longBitsToDouble(Long.parseUnsignedLong(raw,16));
    if(!Double.isFinite(v)||Math.abs(v)>30_000_000.0)throw new IllegalArgumentException("f64 outside finite coordinate domain");return v;
  }
  private static float parseFloat(String raw){
    if(!HEX32.matcher(raw).matches())throw new IllegalArgumentException("expected lowercase raw f32 hex");
    float v=Float.intBitsToFloat((int)Long.parseUnsignedLong(raw,16));
    if(!Float.isFinite(v))throw new IllegalArgumentException("non-finite f32");return v;
  }
  private static String[] exact(String raw,String sep,int n){String[] f=raw.split(sep,-1);if(f.length!=n)throw new IllegalArgumentException("wrong component count");return f;}

  private static State parseState(String raw){
    String[] f=exact(raw,",",18);State s=new State();
    for(int i=0;i<3;i++)s.p[i]=parseDouble(f[i]);for(int i=0;i<3;i++)s.v[i]=parseDouble(f[3+i]);
    for(int i=0;i<6;i++)s.box[i]=parseDouble(f[6+i]);
    s.yOffset=parseFloat(f[12]);s.ySize=parseFloat(f[13]);s.stepHeight=parseFloat(f[14]);s.fallDistance=parseFloat(f[15]);s.yaw=parseFloat(f[16]);
    if(Math.abs(s.v[0])>32||Math.abs(s.v[1])>32||Math.abs(s.v[2])>32)throw new IllegalArgumentException("velocity outside domain");
    if(s.box[3]<=s.box[0]||s.box[4]<=s.box[1]||s.box[5]<=s.box[2]||s.box[3]-s.box[0]>4||s.box[4]-s.box[1]>4||s.box[5]-s.box[2]>4)throw new IllegalArgumentException("body box extents outside domain");
    if(Math.abs(s.yOffset)>2||s.ySize<0||s.ySize>2||s.stepHeight<0||s.stepHeight>1||s.fallDistance<0||s.fallDistance>256||Math.abs(s.yaw)>36000)throw new IllegalArgumentException("f32 state outside domain");
    s.flags=Integer.parseInt(f[17]);if(s.flags<0||s.flags>255||!f[17].matches("0|[1-9][0-9]{0,2}"))throw new IllegalArgumentException("flags outside 0..255");return s;
  }
  private static Scenario parseScenario(String line){
    String[] f=line.split("\t",-1);if(f.length!=5||!ID.matcher(f[0]).matches())throw new IllegalArgumentException("expected five TSV columns and valid case id");
    Scenario s=new Scenario();s.id=f[0];s.initial=parseState(f[1]);
    if(!f[2].equals("-")){
      String[] cells=f[2].split(";",-1);if(cells.length>512)throw new IllegalArgumentException("more than 512 water cells");Set<String> seen=new HashSet<>();
      for(String raw:cells){String[] a=exact(raw,",",5);int x=Integer.parseInt(a[0]),y=Integer.parseInt(a[1]),z=Integer.parseInt(a[2]),id=Integer.parseInt(a[3]),meta=Integer.parseInt(a[4]);
        if(Math.abs(x)>64||y<0||y>127||Math.abs(z)>64||(id!=8&&id!=9)||meta<0||meta>15)throw new IllegalArgumentException("water cell outside domain");
        if(!seen.add(x+","+y+","+z))throw new IllegalArgumentException("duplicate water cell");s.water.add(new WaterCell(x,y,z,id,meta));}
    }
    if(!f[3].equals("-")){String[] boxes=f[3].split(":",-1);if(boxes.length>128)throw new IllegalArgumentException("more than 128 collision boxes");for(String raw:boxes){String[] c=exact(raw,",",6);double[] d=new double[6];for(int i=0;i<6;i++)d[i]=parseDouble(c[i]);if(d[3]<=d[0]||d[4]<=d[1]||d[5]<=d[2])throw new IllegalArgumentException("collider extents must be positive");s.colliders.add(eq.a(d[0],d[1],d[2],d[3],d[4],d[5]));}}
    String[] frames=f[4].split(";",-1);if(frames.length>256)throw new IllegalArgumentException("more than 256 frames");
    for(String raw:frames){String[] a=exact(raw,",",5);Frame frame=new Frame();frame.strafe=parseFloat(a[0]);frame.forward=parseFloat(a[1]);frame.yaw=parseFloat(a[2]);if(Math.abs(frame.strafe)>1||Math.abs(frame.forward)>1||Math.abs(frame.yaw)>36000)throw new IllegalArgumentException("travel input outside domain");if(!a[3].matches("[01]")||!a[4].matches("[01]"))throw new IllegalArgumentException("jump/sneak flags must be 0 or 1");frame.jump=a[3].equals("1");frame.sneak=a[4].equals("1");s.frames.add(frame);}
    if(s.frames.isEmpty())throw new IllegalArgumentException("scenario must contain at least one frame");return s;
  }
  private static List<Scenario> parseInput(byte[] input){
    if(input.length==0||input.length>4*1024*1024||input[input.length-1]!='\n')throw new IllegalArgumentException("input size or final LF invalid");
    String text=new String(input,StandardCharsets.US_ASCII);if(!java.util.Arrays.equals(input,text.getBytes(StandardCharsets.US_ASCII)))throw new IllegalArgumentException("input must be ASCII");
    String[] lines=text.substring(0,text.length()-1).split("\n",-1);if(lines.length>512)throw new IllegalArgumentException("more than 512 scenarios");List<Scenario> scenarios=new ArrayList<>();Set<String> ids=new HashSet<>();long states=0;
    for(int i=0;i<lines.length;i++)try{Scenario s=parseScenario(lines[i]);if(!ids.add(s.id))throw new IllegalArgumentException("duplicate case id");states+=s.frames.size();if(states>20000)throw new IllegalArgumentException("more than 20000 output states");scenarios.add(s);}catch(RuntimeException e){throw new IllegalArgumentException("line "+(i+1)+": "+e.getMessage(),e);}return scenarios;
  }
  private static void setState(SyntheticPlayer e,State s){
    e.e(s.p[0],s.p[1],s.p[2]);e.aM=s.p[0];e.aN=s.p[1];e.aO=s.p[2];e.aP=s.v[0];e.aQ=s.v[1];e.aR=s.v[2];
    e.aW.a=s.box[0];e.aW.b=s.box[1];e.aW.c=s.box[2];e.aW.d=s.box[3];e.aW.e=s.box[4];e.aW.f=s.box[5];
    e.bf=s.yOffset;e.bo=s.ySize;e.bp=s.stepHeight;e.bk=s.fallDistance;e.aS=s.yaw;e.aX=(s.flags&1)!=0;e.aY=(s.flags&2)!=0;e.aZ=(s.flags&4)!=0;e.ba=(s.flags&8)!=0;e.bq=(s.flags&16)!=0;e.bc=(s.flags&32)!=0;e.b(1,(s.flags&64)!=0);e.bd=(s.flags&128)!=0;
  }
  private static int flags(SyntheticPlayer e){return(e.aX?1:0)|(e.aY?2:0)|(e.aZ?4:0)|(e.ba?8:0)|(e.bq?16:0)|(e.bc?32:0)|(e.t()?64:0)|(e.bd?128:0);}
  private static String state(SyntheticPlayer e){eq b=e.aW;return String.join(",",doubleBits(e.aM),doubleBits(e.aN),doubleBits(e.aO),doubleBits(e.aP),doubleBits(e.aQ),doubleBits(e.aR),doubleBits(b.a),doubleBits(b.b),doubleBits(b.c),doubleBits(b.d),doubleBits(b.e),doubleBits(b.f),floatBits(e.bf),floatBits(e.bo),floatBits(e.bp),floatBits(e.bk),floatBits(e.aS),Integer.toString(flags(e)));}

  private static byte[] evaluate(byte[] input,Path auditPath)throws Exception{
    if(!JAR_SHA.equals(sha(Files.readAllBytes(jarPath))))throw new IllegalArgumentException("unexpected original client JAR SHA-256");
    if(uu.m[8].bA!=ln.g||uu.m[9].bA!=ln.g)throw new IllegalStateException("native still/flow water material mapping changed");
    List<Scenario> scenarios=parseInput(input);ByteArrayOutputStream out=new ByteArrayOutputStream();int stateCount=0,inWaterStates=0,eyeWaterStates=0;
    Method update=sn.class.getDeclaredMethod("U");update.setAccessible(true);
    for(Scenario s:scenarios){
      SyntheticWorld world=worldWithoutConstructor(s.water,s.colliders);SyntheticPlayer player=new SyntheticPlayer(world);setState(player,s.initial);
      for(int i=0;i<s.frames.size();i++){
        Frame f=s.frames.get(i);long queriesBefore=player.waterQueries,contactBefore=player.waterContactQueries;
        update.invoke(player);updateCalls++;
        player.aS=f.yaw;player.b(1,f.sneak);
        player.controls(f.jump,f.strafe,f.forward);
        player.o();travelCalls++;
        boolean inWater=player.ag();boolean eyesInWater=player.a(ln.g);
        if(inWater)inWaterStates++;if(eyesInWater)eyeWaterStates++;
        out.write((s.id+"\t"+i+"\t"+state(player)+"\t"+(inWater?1:0)+"\t"+(eyesInWater?1:0)+"\t"+(player.waterQueries-queriesBefore)+"\t"+(player.waterContactQueries-contactBefore)+"\n").getBytes(StandardCharsets.US_ASCII));stateCount++;
      }
    }
    if(worldConstructors!=0)throw new IllegalStateException("synthetic world constructor was invoked");
    writeAudit(auditPath,scenarios.size(),stateCount,inWaterStates,eyeWaterStates);
    return out.toByteArray();
  }
  private static SyntheticWorld worldWithoutConstructor(List<WaterCell> water,List<eq> colliders)throws Exception{
    Field f=Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);SyntheticWorld w=(SyntheticWorld)((Unsafe)f.get(null)).allocateInstance(SyntheticWorld.class);w.water=water;w.colliders=colliders;return w;
  }
  private static String json(String s){return "\""+s.replace("\\","\\\\").replace("\"","\\\"")+"\"";}
  private static String classSha(String name)throws Exception{try(JarFile z=new JarFile(jarPath.toFile())){return sha(z.getInputStream(z.getJarEntry(name+".class")).readAllBytes());}}
  private static void writeAudit(Path path,int caseCount,int stateCount,int inWaterStates,int eyeWaterStates)throws Exception{
    StringBuilder j=new StringBuilder("{\n  \"schema_version\":1,\n  \"suite\":\"water-movement\",\n  \"jar_sha256\":").append(json(JAR_SHA)).append(",\n  \"class_sha256\":{");
    for(int i=0;i<NATIVE_CLASSES.length;i++){String c=NATIVE_CLASSES[i];if(i>0)j.append(',');j.append("\n    ").append(json(c)).append(':').append(json(classSha(c)));}j.append("\n  },\n  \"method_mapping\":{\n    \"water_update\":\"sn.U()V\",\n    \"water_contact\":\"sn.k_()Z\",\n    \"eyes_in_water\":\"sn.a(Lln;)Z\",\n    \"native_fluid_volume_query\":\"fd.a(Leq;Lln;Lsn;)Z\",\n    \"input_acceleration\":\"sn.a(FFF)V\",\n    \"entity_collision_move\":\"sn.b(DDD)V\",\n    \"living_travel\":\"ls.a_(FF)V\",\n    \"player_travel\":\"gs.a_(FF)V\",\n    \"living_input_update\":\"ls.o()V\",\n    \"player_update\":\"gs.o()V\",\n    \"fluid_height\":\"rp.d(I)F\",\n    \"floor_coordinate\":\"in.b(D)I\",\n    \"input_sqrt\":\"in.c(F)F\",\n    \"input_sin\":\"in.a(F)F\",\n    \"input_cos\":\"in.b(F)F\",\n    \"fluid_flow_callback\":\"uu.a(Lfd;IIILsn;Lbt;)V\"\n  },\n  \"field_mapping\":[");
    for(int i=0;i<FIELD_MAPPING.length;i++){if(i>0)j.append(',');j.append(json(FIELD_MAPPING[i]));}j.append("],\n  \"invocations\":{\"entity_update\":").append(updateCalls).append(",\"player_travel\":").append(travelCalls).append(",\"jump\":").append(jumpCalls).append(",\"collision_queries\":").append(collisionQueries).append(",\"water_box_queries\":").append(waterBoxQueries).append(",\"native_water_queries\":").append(nativeWaterQueries).append(",\"water_contact_queries\":").append(nativeWaterContactQueries).append(",\"block_id_queries\":").append(blockIdQueries).append(",\"metadata_queries\":").append(metadataQueries).append("},\n  \"case_count\":").append(caseCount).append(",\n  \"state_count\":").append(stateCount).append(",\n  \"in_water_states\":").append(inWaterStates).append(",\n  \"eyes_in_water_states\":").append(eyeWaterStates).append(",\n  \"native_world_constructors\":").append(worldConstructors).append(",\n  \"environment\":\"synthetic-sparse-water-and-solid-boxes\",\n  \"code_sources\":{");
    for(int i=0;i<NATIVE_CLASSES.length;i++){String c=NATIVE_CLASSES[i];if(i>0)j.append(',');j.append("\n    ").append(json(c)).append(':').append(json(Class.forName(c).getProtectionDomain().getCodeSource().getLocation().toString()));}j.append("\n  }\n}\n");Files.writeString(path,j.toString(),StandardCharsets.US_ASCII);
  }
  private static void bindMethods()throws Exception{
    nativePlayerTravel=gs.class.getDeclaredMethod("a_",float.class,float.class);nativePlayerTravel.setAccessible(true);
    if(nativePlayerTravel.getDeclaringClass()!=gs.class)throw new IllegalStateException("native player travel declaration changed");
    try{SyntheticPlayer.class.getDeclaredMethod("a_",float.class,float.class);throw new IllegalStateException("synthetic player overrides travel");}catch(NoSuchMethodException expected){}
  }
  private static void resetCounters(){travelCalls=jumpCalls=updateCalls=collisionQueries=waterBoxQueries=blockIdQueries=metadataQueries=nativeWaterQueries=nativeWaterContactQueries=0;worldConstructors=0;}
  public static void main(String[] args)throws Exception{
    if(args.length!=2)throw new IllegalArgumentException("usage: WaterMovementOracle <pinned-client.jar> <audit.json>");jarPath=Path.of(args[0]).toRealPath();
    PrintStream original=System.out;System.setOut(new PrintStream(new java.io.OutputStream(){public void write(int b){}},true,StandardCharsets.US_ASCII));
    try{bindMethods();byte[] input=System.in.readAllBytes();byte[] output=evaluate(input,Path.of(args[1]));original.write(output);}catch(Throwable t){System.setOut(original);throw t;}
  }
}
