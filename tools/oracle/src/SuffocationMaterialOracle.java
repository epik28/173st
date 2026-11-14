import java.lang.reflect.Field;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import sun.misc.Unsafe;
/** Native fd.h suffocating material and block cube registry. */
public final class SuffocationMaterialOracle {
 static final class World extends fd {
   int id;
   World() { super((wt)null,"unused",0L); }
   @Override public int a(int x,int y,int z) { return id; }
 }
 public static void main(String[] args) throws Exception {
   if (args.length!=1 || !HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(Path.of(args[0])))).equals(FallHealthOracle.SHA)) throw new IllegalArgumentException("expected pinned client JAR");
   Field f=Unsafe.class.getDeclaredField("theUnsafe"); f.setAccessible(true); World w=(World)((Unsafe)f.get(null)).allocateInstance(World.class);
   java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); System.setOut(out);
   System.out.println("id\tsuffocating");
   for(int id=0;id<256;id++) { w.id=id; System.out.printf("%d\t%b%n",id,w.h(0,0,0)); }
 }
}
