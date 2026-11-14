import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;
/** The native moveEntity contact-fire state transition. */
public final class FireMoveOracle {
  static final class World extends fd {
    boolean hazard;
    World() { super((wt)null, "unused", 0L); }
    @Override public void a(sn entity,String sound,float volume,float pitch) { }
    @Override public boolean t(int x,int y,int z) { return false; }
    @Override public int a(int x,int y,int z) { return 0; }
    @Override public java.util.List a(sn e,eq box) { return java.util.Collections.emptyList(); }
    @Override public boolean a(int a,int b,int c,int d,int e,int f) { return true; }
    @Override public boolean c(eq box) { return hazard; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f=Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u=(Unsafe)f.get(null);
    FireTickOracle.Player p=(FireTickOracle.Player)u.allocateInstance(FireTickOracle.Player.class); World w=(World)u.allocateInstance(World.class); p.aI=w; p.c=new ix(p); p.bx=true;
    Field random=sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p,new Random(0));
    Field box=sn.class.getField("aW"); box.setAccessible(true);
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
    System.out.println("fire\thazard\twet\timmune\tresult\tevents");
    for (int fire : new int[]{-21,-20,-2,-1,0,1,19,20,300}) for(boolean hazard:new boolean[]{false,true}) for(boolean wet:new boolean[]{false,true}) for(boolean immune:new boolean[]{false,true}) {
      box.set(p,eq.b(0.2,1.0,0.2,0.8,2.8,0.8)); p.bv=fire; p.bu=20; p.bx=wet; p.immune(immune); p.events=""; w.hazard=hazard;
      System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); p.b(0.0,0.0,0.0); System.setOut(out);
      System.out.printf("%d\t%b\t%b\t%b\t%d\t%s%n",fire,hazard,wet,immune,p.bv,p.events.isEmpty()?"-":p.events);
    }
  }
}
