import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;

/** Captures the pinned client's sn.a(double,boolean) fall accumulator. */
public final class LandingOracle {
  static final String SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
  static final class Entity extends ls {
    int calls;
    float landed;
    Entity() { super(null); }
    @Override protected void b(float distance) { calls++; landed = distance; }
    void advance(double delta, boolean ground) { super.a(delta, ground); }
    float distance() { return bk; }
  }
  static void emit(String name, Entity entity, double delta, boolean ground) {
    entity.advance(delta, ground);
    System.out.printf("%s\t%016x\t%d\t%08x\t%d\t%08x%n", name,
        Double.doubleToRawLongBits(delta), ground ? 1 : 0,
        Float.floatToRawIntBits(entity.distance()), entity.calls,
        Float.floatToRawIntBits(entity.landed));
  }
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(SHA))
      throw new IllegalArgumentException("expected pinned client JAR");
    Field f = Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true);
    Entity entity = (Entity) ((Unsafe) f.get(null)).allocateInstance(Entity.class);
    System.out.println("case\tdelta_f64_hex\tground\tdistance_f32_hex\tcalls\tlanded_f32_hex");
    emit("fall", entity, -1.25, false);
    emit("fall", entity, -2.5, false);
    emit("fall", entity, 0.75, false);
    emit("fall", entity, -0.25, true);
    emit("fall", entity, -0.25, true);
    emit("fall", entity, -4.125, false);
    emit("fall", entity, 0, true);
  }
}
