import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.*;
import sun.misc.Unsafe;
/** Native living input gating for a dead player. */
public final class DeathInputOracle {
 static final class World extends fd {
  eq collider; boolean liquid;
  World(){super((wt)null,"unused",0L);}
  @Override public int a(int x,int y,int z){return 0;}
  @Override public int e(int x,int y,int z){return 0;}
  @Override public java.util.List a(sn e,eq region){return collider!=null && collider.a(region)?java.util.List.of(collider):java.util.List.of();}
  @Override public java.util.List b(sn e,eq region){return java.util.List.of();}
  @Override public boolean b(eq box){return liquid;}
  @Override public boolean a(int a,int b,int c,int d,int e,int f){return true;}
  @Override public boolean c(eq box){return false;}
  @Override public boolean t(int x,int y,int z){return false;}
  @Override public void a(sn e,String s,float v,float p){}
 }
 static final class Player extends gs {
  Player(){super(null);}
  @Override public void v(){}
  @Override protected void f_(){}
  @Override public void a(vr s,int n){}
  @Override public boolean W(){return false;}
  @Override public boolean ah(){return true;}
  @Override public boolean k_(){return false;}
  void controls(boolean jump,float strafe,float forward){az=jump;aw=strafe;ax=forward;}
  String input(){return aw+"\t"+ax+"\t"+az;}
 }
 public static void main(String[] args)throws Exception {
  if(args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA))throw new IllegalArgumentException("expected pinned client JAR");
  Field f=Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);Unsafe u=(Unsafe)f.get(null);
  Player p=(Player)u.allocateInstance(Player.class);World w=(World)u.allocateInstance(World.class);p.aI=w;p.c=new ix(p);p.bu=20;p.bd=true;p.Y=20;
  Field random=sn.class.getDeclaredField("bs");random.setAccessible(true);random.set(p,new Random(0));Field box=sn.class.getField("aW");box.setAccessible(true);
  java.io.PrintStream out=System.out;System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));Class.forName("uu");Class.forName("jl");System.setOut(out);
  System.out.println("health\tjump_before\tstrafe_before\tforward_before\tstrafe_after\tforward_after\tjump_after");
  for(int health:new int[]{-1,0,1,20}) for(boolean jump:new boolean[]{false,true}) for(float strafe:new float[]{-1,0,1}) for(float forward:new float[]{-1,0,1}){
    double h=(double)(0.6f/2f);box.set(p,eq.b(0.5-h,10,0.5-h,0.5+h,10+(double)1.8f,0.5+h));p.aM=0.5;p.aN=10;p.aO=0.5;p.aP=0;p.aQ=0;p.aR=0;p.aS=0;p.bx=false;p.bv=-20;p.bf=0;p.bo=0;p.bp=0;p.bq=false;p.aX=false;p.bd=true;p.Y=health;p.controls(jump,strafe,forward);w.liquid=false;w.collider=null;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));p.o();System.setOut(out);
    System.out.printf(Locale.ROOT,"%d\t%b\t%s\t%s\t%s%n",health,jump,strafe,forward,p.input());
  }
 }
}
