import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;
/** Lava query sn.ah(), including the native contracted player bounding box. */
public final class FireContactOracle {
  static final class World extends fd {
    int x, y, z, id;
    World() { super((wt)null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return x == this.x && y == this.y && z == this.z ? id : 0; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe)f.get(null);
    FireTickOracle.Player p = (FireTickOracle.Player)u.allocateInstance(FireTickOracle.Player.class); World w = (World)u.allocateInstance(World.class); p.aI = w;
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
    System.out.println("px\tpy\tpz\tx\ty\tz\tid\tlava");
    for (double px : new double[]{-0.3, 0.0, 0.3, 0.5, 0.8, 1.0}) for (double py : new double[]{-0.4, 0.0, 0.6, 1.0}) for (int x : new int[]{-1,0,1}) for (int y : new int[]{-1,0,1,2}) for (int id : new int[]{0, 8, 10, 11}) {
      double h=(double)(0.6F/2.0F); Field box=sn.class.getField("aW"); box.setAccessible(true); box.set(p, eq.b(px-h,py,0.5-h,px+h,py+(double)1.8F,0.5+h));
      w.x=x; w.y=y; w.z=0; w.id=id;
      System.out.printf(java.util.Locale.ROOT, "%s\t%s\t0.5\t%d\t%d\t0\t%d\t%b%n", px,py,x,y,id,p.nativeLava());
    }
  }
}
