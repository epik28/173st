import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;
/** Native player L() opaque-block head test, with a synthetic opaque cell. */
public final class SuffocationOracle {
  static final class World extends fd {
    int x,y,z; boolean solid;
    World() { super((wt)null,"unused",0L); }
    @Override public boolean h(int x,int y,int z) { return solid && x==this.x && y==this.y && z==this.z; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f=Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u=(Unsafe)f.get(null);
    FireTickOracle.Player p=(FireTickOracle.Player)u.allocateInstance(FireTickOracle.Player.class); World w=(World)u.allocateInstance(World.class); p.aI=w; p.bg=0.6f;
    System.out.println("x\tfeet_y\tz\ty_size\tcell_x\tcell_y\tcell_z\tsolid\tsuffocating");
    for(double x:new double[]{-0.3,0.0,0.3,0.5,0.8,1.0}) for(double y:new double[]{-1.5,-0.2,0.0,0.3,1.0}) for(float size:new float[]{0,0.1f}) for(int cx:new int[]{-1,0,1}) for(int cy:new int[]{0,1,2}) for(boolean solid:new boolean[]{false,true}) {
      p.aM=x; p.aN=y+(double)1.62f-(double)size; p.aO=0.5; p.bo=size; w.x=cx;w.y=cy;w.z=0;w.solid=solid;
      System.out.printf(java.util.Locale.ROOT,"%s\t%s\t0.5\t%s\t%d\t%d\t0\t%b\t%b%n",x,y,size,cx,cy,solid,p.L());
    }
  }
}
