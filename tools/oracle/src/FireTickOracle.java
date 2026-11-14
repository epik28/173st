import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.Random;
import sun.misc.Unsafe;
/** Native living tick's fire countdown and lava contact, on synthetic world. */
public final class FireTickOracle {
  static final class World extends fd {
    World() { super((wt) null, "unused", 0L); }
    @Override public int a(int x, int y, int z) { return 0; }
  }
  static final class Player extends gs {
    boolean wet, lava; String events = "";
    Player() { super(null); }
    @Override public void v() { }
    @Override public void a(vr stat, int amount) { }
    @Override protected void ai() { }
    @Override public boolean W() { return false; }
    @Override public boolean k_() { return wet; }
    @Override public boolean ah() { return lava; }
    @Override protected void b(int id, boolean value) { }
    @Override public boolean a(sn attacker, int amount) { events += amount; return true; }
    boolean nativeLava() { return super.ah(); }
    void immune(boolean value) { bC = value; }
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); Unsafe u = (Unsafe) f.get(null);
    Player p = (Player)u.allocateInstance(Player.class); World w = (World)u.allocateInstance(World.class); p.aI=w; p.c=new ix(p); p.bx=true; p.Y=20; p.E=20;
    Field random = sn.class.getDeclaredField("bs"); random.setAccessible(true); random.set(p, new Random(0));
    java.io.PrintStream out = System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("jl"); System.setOut(out);
    System.out.println("fire_before\twet\tlava\timmune\tremote\tfire_after\tdamage_events");
    for (int fire : new int[]{-1,0,1,3,4,19,20,21,40,600}) for (boolean wet : new boolean[]{false,true}) for (boolean lava : new boolean[]{false,true}) for (boolean immune : new boolean[]{false,true}) for (boolean remote : new boolean[]{false,true}) {
      p.bx=true; p.bv=fire; p.wet=wet; p.lava=lava; p.immune(immune); p.events=""; w.B=remote;
      System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); p.U(); System.setOut(out);
      System.out.printf("%d\t%b\t%b\t%b\t%b\t%d\t%s%n", fire,wet,lava,immune,remote,p.bv,p.events.isEmpty()?"-":p.events);
    }
  }
}
