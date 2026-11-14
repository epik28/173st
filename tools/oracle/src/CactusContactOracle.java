import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;
/** Native moveEntity callback for one synthetic cactus block. */
public final class CactusContactOracle {
  static final class World extends fd {
    int x,y,z,id;
    World() { super((wt)null,"unused",0L); }
    @Override public int a(int x,int y,int z) { return x==this.x && y==this.y && z==this.z ? id : 0; }
    @Override public java.util.List a(sn e,eq box) { return java.util.Collections.emptyList(); }
    @Override public boolean a(int a,int b,int c,int d,int e,int f) { return true; }
    @Override public boolean c(eq box) { return false; }
    @Override public boolean t(int x,int y,int z) { return false; }
    @Override public void a(sn e,String sound,float volume,float pitch) { }
  }
  public static void main(String[] args) throws Exception {
    if(args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f=Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u=(Unsafe)f.get(null);
    FireTickOracle.Player p=(FireTickOracle.Player)u.allocateInstance(FireTickOracle.Player.class); World w=(World)u.allocateInstance(World.class); p.aI=w; p.c=new ix(p); p.bu=20;
    Field random=sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p,new Random(0));
    Field box=sn.class.getField("aW"); box.setAccessible(true);
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
    System.out.println("px\tpy\tpz\tx\ty\tz\tid\tdamage_events");
    for(double px:new double[]{-0.3,0,0.3,0.5,0.8,1}) for(double py:new double[]{-0.4,0,0.6,1}) for(int x:new int[]{-1,0,1}) for(int y:new int[]{-1,0,1,2}) for(int id:new int[]{0,1,81}) {
      double h=(double)(0.6f/2f); box.set(p,eq.b(px-h,py,0.5-h,px+h,py+(double)1.8f,0.5+h));
      p.aM=px;p.aN=py;p.aO=0.5;p.bq=false;p.bx=false;p.bv=-20;p.events="";w.x=x;w.y=y;w.z=0;w.id=id;
      System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); p.b(0.0,0.0,0.0); System.setOut(out);
      System.out.printf(java.util.Locale.ROOT,"%s\t%s\t0.5\t%d\t%d\t0\t%d\t%s%n",px,py,x,y,id,p.events.isEmpty()?"-":p.events);
    }
  }
}
