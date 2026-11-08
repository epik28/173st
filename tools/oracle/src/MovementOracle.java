import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.Collections;
import java.util.List;
import java.util.Locale;
import java.util.regex.Pattern;
import java.util.jar.JarFile;
import sun.misc.Unsafe;

/** Invokes the pinned client's own entity movement methods in a synthetic dry world. */
public final class MovementOracle {
  private static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  private static final Pattern ID = Pattern.compile("[A-Za-z0-9_.:-]{1,96}");
  private static final Pattern HEX64 = Pattern.compile("[0-9a-f]{16}");
  private static final Pattern HEX32 = Pattern.compile("[0-9a-f]{8}");
  private static final String[] NATIVE_CLASSES = {"sn", "ls", "eq", "bt", "in", "uu"};
  private static final String[] FIELD_MAPPING = {
      "position_x=aM", "position_y=aN", "position_z=aO",
      "velocity_x=aP", "velocity_y=aQ", "velocity_z=aR",
      "bbox_min_x=aW.a", "bbox_min_y=aW.b", "bbox_min_z=aW.c",
      "bbox_max_x=aW.d", "bbox_max_y=aW.e", "bbox_max_z=aW.f",
      "y_offset=bf", "y_size=bo", "step_height=bp", "fall_distance=bk",
      "yaw_degrees=aS", "on_ground=aX", "collided_horizontally=aY",
      "collided_vertically=aZ", "collided=ba", "no_clip=bq", "in_web=bc",
      "sneaking=t()", "allow_slide=bd"};

  private static long moveCalls, travelCalls, jumpCalls, collisionQueries;
  private static int worldConstructors;
  private static Method nativeMove, nativeTravel, nativeJump;
  private static Path jarPath;

  private static final class State {
    double[] p = new double[3], v = new double[3], box = new double[6];
    float yOffset, ySize, stepHeight, fallDistance, yaw;
    int flags;
  }
  private static final class Frame {
    double[] delta;
    float strafe, forward, yaw;
    boolean jump, sneak;
  }
  private static final class Scenario {
    String id, mode;
    State initial;
    List<eq> colliders = new ArrayList<>();
    int surface;
    List<Frame> frames = new ArrayList<>();
  }

  public static final class SyntheticWorld extends fd {
    List<eq> colliders;
    int surface;
    SyntheticWorld() { super((wt) null, "unused", 0L); worldConstructors++; }
    @Override public List a(sn entity, eq region) {
      collisionQueries++;
      List<eq> hits = new ArrayList<>();
      for (eq collider : colliders) if (collider.a(region)) hits.add(collider);
      return hits;
    }
    @Override public int a(int x, int y, int z) {
      eq voxel = eq.a(x, y, z, x + 1.0, y + 1.0, z + 1.0);
      for (eq collider : colliders) if (collider.a(voxel)) return surface;
      return 0;
    }
    @Override public boolean a(int x1, int y1, int z1, int x2, int y2, int z2) { return true; }
  }

  public static final class SyntheticLiving extends ls {
    SyntheticLiving(fd world) { super(world); }
    @Override public boolean ag() { return false; }
    @Override public boolean ah() { return false; }
    @Override public boolean p() { return false; }
    @Override protected void b(float fallDamage) {}
  }

  private static fd worldWithoutConstructor(List<eq> colliders, int surface) throws Exception {
    Field unsafeField = Unsafe.class.getDeclaredField("theUnsafe");
    unsafeField.setAccessible(true);
    SyntheticWorld world = (SyntheticWorld) ((Unsafe) unsafeField.get(null)).allocateInstance(SyntheticWorld.class);
    world.colliders = colliders;
    world.surface = surface;
    Field entities = fd.class.getDeclaredField("u");
    entities.setAccessible(true);
    entities.set(world, Collections.emptyList());
    return world;
  }

  private static String sha(byte[] bytes) throws Exception {
    return java.util.HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
  }
  private static String doubleBits(double x) { return String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(x)); }
  private static String floatBits(float x) { return String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(x)); }
  private static double parseDouble(String raw) {
    if (!HEX64.matcher(raw).matches()) throw new IllegalArgumentException("expected lowercase raw f64 hex");
    double v = Double.longBitsToDouble(Long.parseUnsignedLong(raw, 16));
    if (!Double.isFinite(v) || Math.abs(v) > 30_000_000.0) throw new IllegalArgumentException("f64 outside finite coordinate domain");
    return v;
  }
  private static float parseFloat(String raw) {
    if (!HEX32.matcher(raw).matches()) throw new IllegalArgumentException("expected lowercase raw f32 hex");
    float v = Float.intBitsToFloat((int) Long.parseUnsignedLong(raw, 16));
    if (!Float.isFinite(v)) throw new IllegalArgumentException("non-finite f32");
    return v;
  }
  private static String[] exact(String raw, String sep, int n) {
    String[] fields = raw.split(sep, -1);
    if (fields.length != n) throw new IllegalArgumentException("wrong component count");
    return fields;
  }

  private static State parseState(String raw) {
    String[] f = exact(raw, ",", 18);
    State s = new State();
    for (int i=0;i<3;i++) s.p[i] = parseDouble(f[i]);
    for (int i=0;i<3;i++) s.v[i] = parseDouble(f[3+i]);
    for (int i=0;i<6;i++) s.box[i] = parseDouble(f[6+i]);
    s.yOffset=parseFloat(f[12]); s.ySize=parseFloat(f[13]); s.stepHeight=parseFloat(f[14]);
    s.fallDistance=parseFloat(f[15]); s.yaw=parseFloat(f[16]);
    if (Math.abs(s.v[0])>32 || Math.abs(s.v[1])>32 || Math.abs(s.v[2])>32) throw new IllegalArgumentException("velocity outside domain");
    if (s.box[3]<=s.box[0] || s.box[4]<=s.box[1] || s.box[5]<=s.box[2] ||
        s.box[3]-s.box[0]>4 || s.box[4]-s.box[1]>4 || s.box[5]-s.box[2]>4) throw new IllegalArgumentException("body box extents outside domain");
    if (Math.abs(s.yOffset)>2 || s.ySize<0 || s.ySize>2 || s.stepHeight<0 || s.stepHeight>1 ||
        s.fallDistance<0 || s.fallDistance>256 || Math.abs(s.yaw)>36000) throw new IllegalArgumentException("f32 state outside domain");
    try { s.flags = Integer.parseInt(f[17]); }
    catch (NumberFormatException e) { throw new IllegalArgumentException("invalid flags", e); }
    if (s.flags<0 || s.flags>255 || !f[17].matches("0|[1-9][0-9]{0,2}")) throw new IllegalArgumentException("flags outside 0..255");
    return s;
  }

  private static Scenario parseScenario(String line) {
    String[] f = line.split("\t", -1);
    if (f.length!=6 || !ID.matcher(f[0]).matches()) throw new IllegalArgumentException("expected six TSV columns and valid case id");
    Scenario s = new Scenario(); s.id=f[0]; s.mode=f[1];
    if (!s.mode.equals("move") && !s.mode.equals("travel")) throw new IllegalArgumentException("mode must be move or travel");
    s.initial=parseState(f[2]);
    if (!f[3].equals("-")) {
      String[] boxes=f[3].split(":",-1);
      if (boxes.length>128) throw new IllegalArgumentException("more than 128 boxes");
      for(String box:boxes) {
        String[] c=exact(box,",",6); double[] d=new double[6];
        for(int i=0;i<6;i++) d[i]=parseDouble(c[i]);
        if(d[3]<=d[0]||d[4]<=d[1]||d[5]<=d[2]) throw new IllegalArgumentException("box extents must be positive");
        s.colliders.add(eq.a(d[0],d[1],d[2],d[3],d[4],d[5]));
      }
    }
    try { s.surface=Integer.parseInt(f[4]); }
    catch(NumberFormatException e) { throw new IllegalArgumentException("invalid surface id",e); }
    if(s.surface!=0&&s.surface!=1&&s.surface!=79) throw new IllegalArgumentException("surface id must be 0, 1, or 79");
    String[] frames=f[5].split(";",-1);
    if(frames.length>256) throw new IllegalArgumentException("more than 256 frames");
    for(String raw:frames) {
      if(raw.isEmpty()) throw new IllegalArgumentException("empty frame");
      Frame frame=new Frame();
      if(s.mode.equals("move")) {
        String[] a=exact(raw,",",3); frame.delta=new double[3];
        for(int i=0;i<3;i++) { frame.delta[i]=parseDouble(a[i]); if(Math.abs(frame.delta[i])>32) throw new IllegalArgumentException("move delta outside domain"); }
      } else {
        String[] a=exact(raw,",",5); frame.strafe=parseFloat(a[0]); frame.forward=parseFloat(a[1]); frame.yaw=parseFloat(a[2]);
        if(Math.abs(frame.strafe)>1||Math.abs(frame.forward)>1||Math.abs(frame.yaw)>36000) throw new IllegalArgumentException("travel input outside domain");
        if(!a[3].equals("0")&&!a[3].equals("1")||!a[4].equals("0")&&!a[4].equals("1")) throw new IllegalArgumentException("frame booleans must be 0 or 1");
        frame.jump=a[3].equals("1"); frame.sneak=a[4].equals("1");
      }
      s.frames.add(frame);
    }
    if(s.frames.isEmpty()) throw new IllegalArgumentException("scenario must contain at least one frame");
    return s;
  }

  private static List<Scenario> parseInput(byte[] input) {
    if(input.length==0||input.length>16*1024*1024||input[input.length-1]!='\n') throw new IllegalArgumentException("input size or final LF invalid");
    String text=new String(input,StandardCharsets.US_ASCII);
    if(!java.util.Arrays.equals(input,text.getBytes(StandardCharsets.US_ASCII))) throw new IllegalArgumentException("input must be ASCII");
    String[] lines=text.substring(0,text.length()-1).split("\n",-1);
    if(lines.length>4096) throw new IllegalArgumentException("more than 4096 scenarios");
    List<Scenario> scenarios=new ArrayList<>(); java.util.HashSet<String> ids=new java.util.HashSet<>(); long states=0;
    for(int i=0;i<lines.length;i++) {
      try {
        Scenario s=parseScenario(lines[i]);
        if(!ids.add(s.id)) throw new IllegalArgumentException("duplicate case id");
        states+=s.frames.size(); if(states>100000) throw new IllegalArgumentException("more than 100000 output states");
        scenarios.add(s);
      } catch(RuntimeException e) { throw new IllegalArgumentException("line "+(i+1)+": "+e.getMessage(),e); }
    }
    return scenarios;
  }

  private static void setState(SyntheticLiving e, State s) {
    e.e(s.p[0],s.p[1],s.p[2]);
    e.aM=s.p[0]; e.aN=s.p[1]; e.aO=s.p[2];
    e.aP=s.v[0]; e.aQ=s.v[1]; e.aR=s.v[2];
    e.aW.a=s.box[0]; e.aW.b=s.box[1]; e.aW.c=s.box[2];
    e.aW.d=s.box[3]; e.aW.e=s.box[4]; e.aW.f=s.box[5];
    e.bf=s.yOffset; e.bo=s.ySize; e.bp=s.stepHeight; e.bk=s.fallDistance; e.aS=s.yaw;
    e.aX=(s.flags&1)!=0; e.aY=(s.flags&2)!=0; e.aZ=(s.flags&4)!=0; e.ba=(s.flags&8)!=0;
    e.bq=(s.flags&16)!=0; e.bc=(s.flags&32)!=0; e.b(1,(s.flags&64)!=0); e.bd=(s.flags&128)!=0;
  }

  private static int flags(SyntheticLiving e) {
    return (e.aX?1:0)|(e.aY?2:0)|(e.aZ?4:0)|(e.ba?8:0)|(e.bq?16:0)|(e.bc?32:0)|(e.t()?64:0)|(e.bd?128:0);
  }
  private static String state(SyntheticLiving e) {
    eq b=e.aW;
    return String.join(",",doubleBits(e.aM),doubleBits(e.aN),doubleBits(e.aO),doubleBits(e.aP),doubleBits(e.aQ),doubleBits(e.aR),
        doubleBits(b.a),doubleBits(b.b),doubleBits(b.c),doubleBits(b.d),doubleBits(b.e),doubleBits(b.f),
        floatBits(e.bf),floatBits(e.bo),floatBits(e.bp),floatBits(e.bk),floatBits(e.aS),Integer.toString(flags(e)));
  }

  private static byte[] evaluate(byte[] input, Path auditPath, byte[] registryStdout) throws Exception {
    if(!JAR_SHA.equals(sha(Files.readAllBytes(jarPath)))) throw new IllegalArgumentException("unexpected original client JAR SHA-256");
    List<Scenario> scenarios=parseInput(input); // validate every row before any native simulation
    bindOriginalMethods();
    int stoneId=uu.u.bn, iceId=uu.aU.bn;
    float stoneSlip=uu.u.bB, iceSlip=uu.aU.bB;
    if(stoneId!=1||iceId!=79) throw new IllegalStateException("unexpected native stone/ice ids");
    ByteArrayOutputStream output=new ByteArrayOutputStream();
    int stateCount=0;
    for(Scenario s:scenarios) {
      SyntheticLiving entity=new SyntheticLiving(worldWithoutConstructor(s.colliders,s.surface));
      setState(entity,s.initial);
      for(int i=0;i<s.frames.size();i++) {
        Frame f=s.frames.get(i);
        if(s.mode.equals("move")) {
          nativeMove.invoke(entity,f.delta[0],f.delta[1],f.delta[2]); moveCalls++;
        } else {
          entity.aS=f.yaw; entity.b(1,f.sneak);
          if(f.jump&&entity.aX) { nativeJump.invoke(entity); jumpCalls++; }
          nativeTravel.invoke(entity,f.strafe,f.forward); travelCalls++;
        }
        output.write((s.id+"\t"+i+"\t"+state(entity)+"\n").getBytes(StandardCharsets.US_ASCII));
        stateCount++;
      }
    }
    if(worldConstructors!=0) throw new IllegalStateException("fixture world constructor was unexpectedly invoked");
    writeAudit(auditPath,scenarios.size(),stateCount,stoneSlip,iceSlip,registryStdout);
    return output.toByteArray();
  }

  private static void bindOriginalMethods() throws Exception {
    Class<?> sn=Class.forName("sn"), ls=Class.forName("ls");
    nativeMove=sn.getDeclaredMethod("b",double.class,double.class,double.class);
    nativeTravel=ls.getDeclaredMethod("a_",float.class,float.class);
    nativeJump=ls.getDeclaredMethod("R"); nativeJump.setAccessible(true);
    if(nativeMove.getDeclaringClass()!=sn||nativeTravel.getDeclaringClass()!=ls||nativeJump.getDeclaringClass()!=ls)
      throw new IllegalStateException("unexpected native declaring class");
    for(Method m:new Method[]{sn.getDeclaredMethod("a",float.class,float.class,float.class),nativeMove,
                              nativeTravel,nativeJump}) {
      if(m.getDeclaringClass()!=sn&&m.getDeclaringClass()!=ls) throw new IllegalStateException("non-native motion method");
    }
    try { SyntheticLiving.class.getDeclaredMethod("a_",float.class,float.class); throw new IllegalStateException("travel override detected"); }
    catch(NoSuchMethodException expected) {}
    try { SyntheticLiving.class.getDeclaredMethod("b",double.class,double.class,double.class); throw new IllegalStateException("move override detected"); }
    catch(NoSuchMethodException expected) {}
    try { SyntheticLiving.class.getDeclaredMethod("R"); throw new IllegalStateException("jump override detected"); }
    catch(NoSuchMethodException expected) {}
    try { SyntheticLiving.class.getDeclaredMethod("a",float.class,float.class,float.class); throw new IllegalStateException("acceleration override detected"); }
    catch(NoSuchMethodException expected) {}
  }

  private static String descriptor(Method m) {
    StringBuilder d=new StringBuilder("(");
    for(Class<?> p:m.getParameterTypes()) d.append(typeDescriptor(p));
    return d.append(')').append(typeDescriptor(m.getReturnType())).toString();
  }
  private static String typeDescriptor(Class<?> c) {
    if(c==double.class)return "D"; if(c==float.class)return "F"; if(c==int.class)return "I";
    if(c==boolean.class)return "Z"; if(c==void.class)return "V";
    return "L"+c.getName().replace('.','/')+";";
  }
  private static String classSha(String name) throws Exception {
    try(JarFile archive=new JarFile(jarPath.toFile())) {
      return sha(archive.getInputStream(archive.getJarEntry(name+".class")).readAllBytes());
    }
  }
  private static String jsonString(String s) { return "\""+s.replace("\\","\\\\").replace("\"","\\\"")+"\""; }
  private static void writeAudit(Path path,int caseCount,int stateCount,float stoneSlip,float iceSlip,byte[] registryStdout) throws Exception {
    StringBuilder j=new StringBuilder("{\n  \"schema_version\":1,\n  \"suite\":\"movement\",\n");
    j.append("  \"jar_sha256\":").append(jsonString(JAR_SHA)).append(",\n  \"class_sha256\":{");
    for(int i=0;i<NATIVE_CLASSES.length;i++) {
      String c=NATIVE_CLASSES[i]; Class<?> clazz=Class.forName(c);
      Path source=Path.of(clazz.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
      if(!source.equals(jarPath)) throw new IllegalStateException("class loaded outside pinned JAR: "+c);
      if(i>0)j.append(','); j.append("\n    ").append(jsonString(c)).append(':').append(jsonString(classSha(c)));
    }
    j.append("\n  },\n  \"code_sources\":{");
    for(int i=0;i<NATIVE_CLASSES.length;i++){if(i>0)j.append(',');j.append("\n    ").append(jsonString(NATIVE_CLASSES[i])).append(':').append(jsonString(jarPath.toUri().toString()));}
    j.append("\n  },\n  \"method_mapping\":{");
    Method[] methods={nativeMove,nativeTravel,nativeJump}; String[] keys={"move","travel","jump"};
    for(int i=0;i<methods.length;i++) {
      if(i>0)j.append(','); Method m=methods[i];
      j.append("\n    ").append(jsonString(keys[i])).append(":{\"class\":").append(jsonString(m.getDeclaringClass().getName()))
          .append(",\"name\":").append(jsonString(m.getName())).append(",\"descriptor\":").append(jsonString(descriptor(m))).append('}');
    }
    j.append("\n  },\n  \"field_mapping\":{");
    for(int i=0;i<FIELD_MAPPING.length;i++){if(i>0)j.append(',');String[] p=FIELD_MAPPING[i].split("=",2);j.append("\n    ").append(jsonString(p[0])).append(':').append(jsonString(p[1]));}
    j.append("\n  },\n  \"invocations\":{"); int n=0;
    long[] counts={moveCalls,travelCalls,jumpCalls};
    for(int i=0;i<counts.length;i++)if(counts[i]>0){if(n++>0)j.append(',');j.append("\n    ").append(jsonString(keys[i])).append(':').append(counts[i]);}
    j.append("\n  },\n  \"surfaces\":{\n    \"1\":").append(jsonString(floatBits(stoneSlip)))
        .append(",\n    \"79\":").append(jsonString(floatBits(iceSlip))).append("\n  },\n")
        .append("  \"case_count\":").append(caseCount).append(",\n  \"state_count\":").append(stateCount).append(",\n")
        .append("  \"collision_queries\":").append(collisionQueries).append(",\n  \"native_world_constructors\":").append(worldConstructors)
        .append(",\n  \"environment\":\"synthetic-static-dry\",\n  \"registry_stdout_bytes\":").append(registryStdout.length)
        .append(",\n  \"registry_stdout_sha256\":").append(jsonString(sha(registryStdout))).append("\n}\n");
    Files.write(Path.of(path.toString()+".registry.stdout"),registryStdout);
    Files.writeString(path,j,StandardCharsets.US_ASCII);
  }

  public static void main(String[] args) {
    PrintStream original=System.out;
    ByteArrayOutputStream startup=new ByteArrayOutputStream();
    try {
      if(args.length!=2) throw new IllegalArgumentException("usage: MovementOracle CLIENT_JAR AUDIT_PATH");
      jarPath=Path.of(args[0]).toRealPath(); Path auditPath=Path.of(args[1]);
      System.setOut(new PrintStream(startup,true,StandardCharsets.UTF_8));
      byte[] input=System.in.readNBytes(16*1024*1024+1);
      byte[] output=evaluate(input,auditPath,startup.toByteArray());
      System.setOut(original);
      System.out.write(output); System.out.flush();
    } catch(Throwable failure) {
      System.setOut(original);
      Throwable cause=failure instanceof java.lang.reflect.InvocationTargetException && failure.getCause()!=null?failure.getCause():failure;
      System.err.println("movement oracle error: "+cause.getMessage());
      System.exit(2);
    } finally { System.setOut(original); }
  }
}
