import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native yw item use against player health; no world or save is loaded. */
public final class FoodOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class Player extends gs { Player() { super(null); } @Override public void v() { } }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA)) throw new IllegalArgumentException("wrong client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Player p = (Player) ((Unsafe) f.get(null)).allocateInstance(Player.class);
    java.io.PrintStream output = System.out;
    System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl"); Class.forName("gm");
    System.setOut(output);
    System.out.println("id\theal_amount\tmax_stack\thealth_before\tcount_before\ttimer_before\thealth_after\tcount_after\ttimer_after\treturned_id\treturned_count");
    for (int id=0; id<gm.c.length; id++) if (gm.c[id] instanceof yw food) {
      for (int health : new int[]{0, 1, 10, 19, 20}) {
        iz stack = new iz(id, 1, 0); p.Y=health; p.E=20; p.by=3;
        iz returned = food.a(stack, (fd) null, p);
        System.out.printf("%d\t%d\t%d\t%d\t1\t3\t%d\t%d\t%d\t%d\t%d%n", id, food.l(), stack.c(), health, p.Y, stack.a, p.by, returned.c, returned.a);
      }
    }
  }
}
