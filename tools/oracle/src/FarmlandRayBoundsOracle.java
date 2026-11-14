import java.nio.file.Files;
import java.nio.file.Path;

/** Native selection bounds set by the Beta farmland block. */
public final class FarmlandRayBoundsOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !PlacementOracle.sha(Files.readAllBytes(Path.of(args[0]))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); uu b=uu.m[60]; System.setOut(out);
    System.out.println("id\tmin_x\tmin_y\tmin_z\tmax_x\tmax_y\tmax_z");
    System.out.printf("60\t%s\t%s\t%s\t%s\t%s\t%s%n",b.bs,b.bt,b.bu,b.bv,b.bw,b.bx);
  }
}
