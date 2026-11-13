import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native ob.a(gs,fd,iz) selected-slot use, without world or save access. */
public final class FoodRouteOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    FoodOracle.Player p = (FoodOracle.Player) u.allocateInstance(FoodOracle.Player.class);
    ob controller = (ob) u.allocateInstance(ob.class); p.c = new ix(p); p.E = 20;
    java.io.PrintStream out = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl");
    System.setOut(out);
    System.out.println("id\tcount_before\thealth_before\ttimer_before\tused\thealth_after\ttimer_after\tslot_after");
    for (int id : new int[]{260, 282, 357}) for (int health : new int[]{10, 20}) {
      int count = id == 357 ? 8 : 1; p.c.c = 4; p.c.a[4] = new iz(id,count,0); p.Y=health; p.by=3;
      boolean used = controller.a(p, null, p.c.a[4]); iz result=p.c.a[4];
      System.out.printf("%d\t%d\t%d\t3\t%d\t%d\t%d\t%s%n", id, count, health, used ? 1 : 0, p.Y, p.by, result==null ? "-" : result.c+":"+result.a);
    }
  }
}
