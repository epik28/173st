import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Native gs.b(int) damage reduction, using controlled inventory armor scores. */
public final class ArmorDamageOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class Armor extends ix {
    int score, wear;
    Armor(gs owner) { super(owner); }
    @Override public int f() { return score; }
    @Override public void e(int damage) { wear = damage; }
  }
  static final class Player extends gs {
    Player() { super(null); }
    @Override public void v() { }
    void apply(int damage) { super.b(damage); }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Unsafe u = (Unsafe) f.get(null);
    Field carry = gs.class.getDeclaredField("bO"); carry.setAccessible(true);
    System.out.println("case\tscore\tdamage\thealth_before\thealth_after\tcarry_before\tcarry_after\twear_call");
    for (int score : new int[] {0, 1, 5, 10, 20}) {
      Player p = (Player) u.allocateInstance(Player.class); p.Y = 100;
      Armor armor = new Armor(p); armor.score = score; p.c = armor;
      for (int damage : new int[] {1, 1, 3, 7}) {
        int hp = p.Y, old = carry.getInt(p);
        p.apply(damage);
        System.out.printf("armor_%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d%n", score, score, damage, hp, p.Y, old, carry.getInt(p), armor.wear);
      }
    }
  }
}
