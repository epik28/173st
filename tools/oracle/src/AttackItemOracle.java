import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native held-item attack damage and wear, without world or save access. */
public final class AttackItemOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    FoodOracle.Player player = (FoodOracle.Player) u.allocateInstance(FoodOracle.Player.class);
    FoodOracle.Player target = (FoodOracle.Player) u.allocateInstance(FoodOracle.Player.class);
    player.c = new ix(player);
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl");
    System.setOut(out);
    System.out.println("id\tattack\tmax_damage\tdamage_after_hit\tcount_after_hit");
    for (int id=256; id<=359; id++) if (gm.c[id] != null) {
      iz stack=new iz(id,1,0); player.c.a[0]=stack;
      int attack=player.c.a(target), max=stack.j();
      stack.a(target,player);
      System.out.printf("%d\t%d\t%d\t%d\t%d%n",id,attack,max,stack.h(),stack.a);
    }
  }
}
