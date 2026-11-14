import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;
/** Native player lava travel with empty collision list and no jumping. */
public final class LavaTravelOracle {
 static final class World extends fd {
  World() { super((wt)null,"unused",0L); }
  @Override public int a(int x,int y,int z) { return 0; }
  @Override public int e(int x,int y,int z) { return 0; }
  @Override public java.util.List a(sn e,eq box) { return java.util.Collections.emptyList(); }
  @Override public boolean a(int a,int b,int c,int d,int e,int f) { return true; }
  @Override public boolean c(eq box) { return false; }
  @Override public boolean t(int x,int y,int z) { return false; }
  @Override public void a(sn e,String s,float v,float p) { }
 }
 public static void main(String[] args) throws Exception {
  if(args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
  Field f=Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u=(Unsafe)f.get(null);
  FireTickOracle.Player p=(FireTickOracle.Player)u.allocateInstance(FireTickOracle.Player.class); World w=(World)u.allocateInstance(World.class); p.aI=w;p.c=new ix(p);p.bu=20;p.bd=true;
  Field random=sn.class.getDeclaredField("bs");random.setAccessible(true);random.set(p,new Random(0));Field box=sn.class.getField("aW");box.setAccessible(true);
  java.io.PrintStream out=System.out;System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));Class.forName("uu");Class.forName("jl");System.setOut(out);
  System.out.println("vx\tvy\tvz\tstrafe\tforward\tyaw\tpx_after\tpy_after\tpz_after\tvx_after\tvy_after\tvz_after");
  for(double vx:new double[]{-0.1,0.0,0.1}) for(double vy:new double[]{-0.2,0.0,0.2}) for(float fwd:new float[]{-1,0,1}) for(float yaw:new float[]{0,45,180}) {
   double h=(double)(0.6f/2f);box.set(p,eq.b(0.5-h,10,0.5-h,0.5+h,10+(double)1.8f,0.5+h));p.aM=0.5;p.aN=10;p.aO=0.5;p.aP=vx;p.aQ=vy;p.aR=0.05;p.aS=yaw;p.lava=true;p.bx=false;p.bv=-20;p.bf=0;p.bo=0;p.bp=0;p.bq=false;p.aX=false;p.bd=true;p.events="";
   System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));p.a_(0.0f,fwd);System.setOut(out);
   System.out.printf(java.util.Locale.ROOT,"%s\t%s\t0.05\t0.0\t%s\t%s\t%s\t%s\t%s\t%s\t%s\t%s%n",vx,vy,fwd,yaw,p.aM,p.aN,p.aO,p.aP,p.aQ,p.aR);
  }
 }
}
