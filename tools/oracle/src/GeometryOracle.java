import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.lang.reflect.Field;
import java.lang.reflect.Method;
import java.net.URISyntaxException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HashSet;
import java.util.LinkedHashMap;
import java.util.List;
import java.util.Locale;
import java.util.Map;
import java.util.Set;
import java.util.regex.Pattern;
import java.util.jar.JarFile;

/** Calls the pinned Beta client geometry classes directly through reflection. */
public final class GeometryOracle {
  private static final String JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  private static final Pattern HEX = Pattern.compile("[0-9a-f]{16}");
  private static final Map<String, Method> METHODS = new LinkedHashMap<>();
  private static final Map<String, Long> COUNTS = new LinkedHashMap<>();
  private static final Map<String, String> DESCRIPTORS = new LinkedHashMap<>();
  private static Method boxPoolClear, vecPoolClear;

  private static Method bind(String key, String owner, String name, Class<?>... args) throws Exception {
    Method m = Class.forName(owner).getDeclaredMethod(name, args);
    if (!java.lang.reflect.Modifier.isPublic(m.getModifiers())) throw new IllegalStateException("native method not public: " + key);
    METHODS.put(key, m);
    COUNTS.put(key, 0L);
    DESCRIPTORS.put(key, owner + "." + name + descriptor(m));
    return m;
  }

  private static String descriptor(Method m) {
    StringBuilder s = new StringBuilder("(");
    for (Class<?> p : m.getParameterTypes()) s.append(typeDescriptor(p));
    return s.append(')').append(typeDescriptor(m.getReturnType())).toString();
  }

  private static String typeDescriptor(Class<?> c) {
    if (c == double.class) return "D";
    if (c == float.class) return "F";
    if (c == int.class) return "I";
    if (c == boolean.class) return "Z";
    if (c == void.class) return "V";
    return "L" + c.getName().replace('.', '/') + ";";
  }

  private static Object call(String key, Object target, Object... args) throws Exception {
    Object result;
    try { result = METHODS.get(key).invoke(target, args); }
    catch (java.lang.reflect.InvocationTargetException e) {
      Throwable cause = e.getCause();
      if (cause instanceof Exception x) throw x;
      if (cause instanceof Error x) throw x;
      throw e;
    }
    COUNTS.put(key, COUNTS.get(key) + 1); // only after the native Method.invoke returned
    return result;
  }

  private static double number(String raw) {
    if (!HEX.matcher(raw).matches()) throw new IllegalArgumentException("expected lowercase raw f64 hex");
    double x = Double.longBitsToDouble(Long.parseUnsignedLong(raw, 16));
    if (!Double.isFinite(x) || Math.abs(x) > 30_000_000.0) throw new IllegalArgumentException("number outside finite domain");
    return x;
  }

  private static String bits(double x) { return String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(x)); }
  private static Object factoryBox(double... n) throws Exception { return METHODS.get("box_new").invoke(null,n[0],n[1],n[2],n[3],n[4],n[5]); }
  private static Object factoryVec(double x, double y, double z) throws Exception { return METHODS.get("vec_new").invoke(null,x,y,z); }
  private static double field(Object o, String n) throws Exception { return o.getClass().getField(n).getDouble(o); }
  private static String box(Object o) throws Exception {
    return String.join(",", bits(field(o,"a")), bits(field(o,"b")), bits(field(o,"c")), bits(field(o,"d")), bits(field(o,"e")), bits(field(o,"f")));
  }
  private static String vec(Object o) throws Exception { return String.join(",", bits(field(o,"a")), bits(field(o,"b")), bits(field(o,"c"))); }
  private static double[] nums(String[] a, int start, int count) {
    if (a.length != start + count) throw new IllegalArgumentException("arity");
    double[] v = new double[count]; for (int i=0;i<count;i++) v[i]=number(a[start+i]); return v;
  }
  private static Object makeBox(double[] n, int start) throws Exception {
    return factoryBox(n[start], n[start+1], n[start+2], n[start+3], n[start+4], n[start+5]);
  }
  private static Object makeVec(double[] n, int start) throws Exception { return factoryVec(n[start],n[start+1],n[start+2]); }

  private static String eval(String op, String[] raw) throws Exception {
    if (op.startsWith("box_")) {
      if (op.equals("box_new")) { double[] n=nums(raw,0,6); return box(call("box_new",null,n[0],n[1],n[2],n[3],n[4],n[5])); }
      if (op.equals("box_sweep") || op.equals("box_expand") || op.equals("box_offset")) {
        double[] n=nums(raw,0,9); Object b=makeBox(n,0); String key=op;
        return box(call(key,b,n[6],n[7],n[8]));
      }
      if (op.startsWith("box_clip_")) {
        double[] n=nums(raw,0,13); Object obstacle=makeBox(n,0), moving=makeBox(n,6);
        return bits((double)call(op,obstacle,moving,n[12]));
      }
      if (op.equals("box_intersects")) { double[] n=nums(raw,0,12); return (boolean)call(op,makeBox(n,0),makeBox(n,6))?"1":"0"; }
      if (op.equals("box_contains")) { double[] n=nums(raw,0,9); return (boolean)call(op,makeBox(n,0),makeVec(n,6))?"1":"0"; }
      if (op.equals("box_average_edge")) { double[] n=nums(raw,0,6); return bits((double)call(op,makeBox(n,0))); }
      if (op.equals("box_ray")) {
        double[] n=nums(raw,0,12); Object hit=call(op,makeBox(n,0),makeVec(n,6),makeVec(n,9));
        if (hit==null) return "none";
        int face=hit.getClass().getField("e").getInt(hit); Object point=hit.getClass().getField("f").get(hit);
        return face+","+vec(point);
      }
    }
    if (op.startsWith("vec_")) {
      if (op.equals("vec_new")) { double[] n=nums(raw,0,3); return vec(call("vec_new",null,n[0],n[1],n[2])); }
      if (op.equals("vec_to") || op.equals("vec_cross") || op.equals("vec_distance") || op.equals("vec_distance_sq")) {
        double[] n=nums(raw,0,6); Object self=makeVec(n,0), other=makeVec(n,3);
        if (op.equals("vec_to") || op.equals("vec_cross")) return vec(call(op,self,other));
        return bits((double)call(op,self,other));
      }
      if (op.equals("vec_add")) { double[] n=nums(raw,0,6); return vec(call(op,makeVec(n,0),n[3],n[4],n[5])); }
      if (op.equals("vec_normalize") || op.equals("vec_length")) {
        Object v=makeVec(nums(raw,0,3),0); return op.equals("vec_length")?bits((double)call(op,v)):vec(call(op,v));
      }
      if (op.startsWith("vec_intermediate_")) {
        double[] n=nums(raw,0,7); Object start=makeVec(n,0), end=makeVec(n,3);
        Object result=call(op,start,end,n[6]); return result==null?"none":vec(result);
      }
    }
    throw new IllegalArgumentException("unknown operation");
  }

  private static String sha(byte[] data) throws Exception {
    return java.util.HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(data));
  }
  private static String classSha(String name, Path jar) throws Exception {
    try (JarFile jf = new JarFile(jar.toFile())) { return sha(jf.getInputStream(jf.getJarEntry(name.replace('.','/')+".class")).readAllBytes()); }
  }
  private static Path codeSource(Class<?> c) throws Exception { return Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath(); }

  private static void audit(Path jar, Path output, int caseCount) throws Exception {
    StringBuilder j=new StringBuilder();
    j.append("{\n  \"schema_version\":1,\n  \"suite\":\"geometry\",\n  \"jar_sha256\":\"").append(JAR_SHA).append("\",\n");
    j.append("  \"class_sha256\":{"); int i=0;
    for(String c:List.of("eq","bt","vf","in")) {
      Class<?> cls=Class.forName(c); Path src=codeSource(cls);
      if(!src.equals(jar)) throw new IllegalStateException("class loaded from unexpected code source: "+c+" -> "+src);
      if(i++>0)j.append(",\n");
      j.append("\n    \"").append(c).append("\":\"").append(classSha(c,jar)).append("\"");
    }
    j.append("\n  },\n  \"code_sources\":{");i=0;
    for(String c:List.of("eq","bt","vf","in")){if(i++>0)j.append(',');j.append("\n    \"").append(c).append("\":\"").append(codeSource(Class.forName(c)).toUri()).append("\"");}
    j.append("\n  },\n  \"method_mapping\":{"); i=0;
    for(var e:DESCRIPTORS.entrySet()){
      if(i++>0)j.append(',');
      String[] parts=e.getValue().split("\\.",2); String owner=parts[0], tail=parts[1]; int p=tail.indexOf('(');
      j.append("\n    \"").append(e.getKey()).append("\":{\"class\":\"").append(owner).append("\",\"name\":\"").append(tail,0,p).append("\",\"descriptor\":\"").append(tail.substring(p)).append("\"}");
    }
    j.append("\n  },\n  \"invocations\":{");i=0;
    for(var e:COUNTS.entrySet()){if(e.getValue()==0)continue;if(i++>0)j.append(',');j.append("\n    \"").append(e.getKey()).append("\":").append(e.getValue());}
    j.append("\n  },\n  \"case_count\":").append(caseCount).append("\n}\n"); Files.writeString(output,j,StandardCharsets.US_ASCII);
  }

  public static void main(String[] args) throws Exception {
    if(args.length!=2)throw new IllegalArgumentException("usage: GeometryOracle CLIENT_JAR AUDIT_PATH");
    Path jar=Path.of(args[0]).toRealPath(), audit=Path.of(args[1]);
    if(!JAR_SHA.equals(sha(Files.readAllBytes(jar))))throw new IllegalArgumentException("unexpected client JAR SHA-256");
    bind("box_new","eq","a",double.class,double.class,double.class,double.class,double.class,double.class);
    bind("box_sweep","eq","a",double.class,double.class,double.class);
    bind("box_expand","eq","b",double.class,double.class,double.class);
    bind("box_offset","eq","c",double.class,double.class,double.class);
    bind("box_clip_x","eq","a",Class.forName("eq"),double.class);
    bind("box_clip_y","eq","b",Class.forName("eq"),double.class);
    bind("box_clip_z","eq","c",Class.forName("eq"),double.class);
    bind("box_intersects","eq","a",Class.forName("eq"));
    bind("box_contains","eq","a",Class.forName("bt"));
    bind("box_average_edge","eq","c");
    bind("box_ray","eq","a",Class.forName("bt"),Class.forName("bt"));
    bind("vec_new","bt","a",double.class,double.class,double.class);
    bind("vec_to","bt","a",Class.forName("bt"));
    bind("vec_cross","bt","b",Class.forName("bt"));
    bind("vec_add","bt","c",double.class,double.class,double.class);
    bind("vec_normalize","bt","c");
    bind("vec_length","bt","d");
    bind("vec_distance","bt","c",Class.forName("bt"));
    bind("vec_distance_sq","bt","d",Class.forName("bt"));
    bind("vec_intermediate_x","bt","a",Class.forName("bt"),double.class);
    bind("vec_intermediate_y","bt","b",Class.forName("bt"),double.class);
    bind("vec_intermediate_z","bt","c",Class.forName("bt"),double.class);
    boxPoolClear=Class.forName("eq").getDeclaredMethod("a");
    vecPoolClear=Class.forName("bt").getDeclaredMethod("a");
    List<String> outputs=new ArrayList<>(); Set<String> ids=new HashSet<>();
    byte[] input=System.in.readAllBytes();
    if(input.length>32*1024*1024 || input.length==0 || input[input.length-1]!='\n')throw new IllegalArgumentException("input size/final LF");
    String text=new String(input,StandardCharsets.US_ASCII);
    if(!java.util.Arrays.equals(input,text.getBytes(StandardCharsets.US_ASCII)))throw new IllegalArgumentException("input must be ASCII");
    String[] lines=text.substring(0,text.length()-1).split("\n",-1);
    if(lines.length>100000)throw new IllegalArgumentException("too many rows");
    for(int line=0;line<lines.length;line++) {
      String[] f=lines[line].split("\t",-1);
      if(f.length<3||!f[0].matches("[A-Za-z0-9_.:-]{1,96}")||!ids.add(f[0]))throw new IllegalArgumentException("invalid row/id at "+(line+1));
      try {
        boxPoolClear.invoke(null); vecPoolClear.invoke(null);
        outputs.add(f[0]+"\t"+eval(f[1],java.util.Arrays.copyOfRange(f,2,f.length))+"\n");
      }
      catch(Exception e){throw new IllegalArgumentException("row "+(line+1)+" ("+f[1]+"): "+e.getMessage(),e);}
    }
    audit(jar,audit,lines.length);
    for(String out:outputs)System.out.print(out);
  }
}
