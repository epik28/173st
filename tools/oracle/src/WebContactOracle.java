import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.*;
import java.util.*;
import sun.misc.Unsafe;
/** Native moveEntity block contact scan against a synthetic world containing one web. */
public final class WebContactOracle {
 static final class World extends fd {
  World() { super((wt)null,"unused",0L); }
  @Override public void a(sn e,String s,float v,float p) {}
  @Override public boolean t(int x,int y,int z){return false;}
  @Override public int a(int x,int y,int z){return x==0 && y==1 && z==0 ? 30:0;}
  @Override public List a(sn e,eq box){return Collections.emptyList();}
  @Override public boolean a(int a,int b,int c,int d,int e,int f){return true;}
  @Override public boolean c(eq box){return false;}
 }
 static final class Player extends gs {
  Player(){super(null);}
  @Override public void v() {}
  @Override public void a(vr stat,int amount) {}
  @Override protected void ai() {}
  @Override public boolean W(){return false;}
  @Override public boolean k_(){return false;}
  @Override public boolean ah(){return false;}
  @Override protected void b(int id,boolean value){}
 }
 public static void main(String[] args)throws Exception {
  if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");
  Path jar=Path.of(args[0]).toRealPath();
  if(!HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar))).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  Field f=Unsafe.class.getDeclaredField("theUnsafe");f.setAccessible(true);Unsafe unsafe=(Unsafe)f.get(null);
  Player p=(Player)unsafe.allocateInstance(Player.class);World w=(World)unsafe.allocateInstance(World.class);p.aI=w;p.c=new ix(p);p.bx=true;
  Field random=sn.class.getDeclaredField("bs");random.setAccessible(true);random.set(p,new Random(0));
  Field box=sn.class.getField("aW");box.setAccessible(true);
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));Class.forName("uu");System.setOut(out);
  for(double edge:new double[]{-0.01,0.0,0.0005,0.001,0.0011,0.5,0.9989,0.999,0.9995,1.0,1.001}) {
   p.bc=false;box.set(p,eq.b(edge-0.25,1.1,0.25,edge,1.9,0.75));
   System.setOut(new PrintStream(OutputStream.nullOutputStream()));p.b(0.0,0.0,0.0);System.setOut(out);
   out.printf(Locale.ROOT,"%.4f\t%b%n",edge,p.bc);
  }
 }
}
