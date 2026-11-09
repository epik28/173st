import java.io.IOException;
import java.io.ByteArrayOutputStream;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.util.List;

/** Synthetic native edit probes using the bounded world/light helpers in LightUpdateOracle. */
public final class LightMaterialEditOracle {
  static final String REQUEST = "material-edits-v1\n";
  record Cell(int x, int y, int z, int id) {}
  record Point(int x, int y, int z) {}
  record Case(String name, List<Cell> seed, Cell edit, int replacement, List<Point> samples) {}

  static void put(LightUpdateOracle.W world, Cell cell) throws Exception {
    if (!world.a(cell.x, cell.y, cell.z, cell.id, 0))
      throw new IOException("native seed placement failed at " + cell.x + "," + cell.y + "," + cell.z);
    LightUpdateOracle.settle(world, cell.x, cell.y, cell.z);
  }

  static Case[] cases() {
    return new Case[] {
      new Case("canopy-log-remove", List.of(
          new Cell(8, 64, 8, 17), new Cell(8, 65, 8, 17), new Cell(8, 66, 8, 17),
          new Cell(7, 66, 8, 18), new Cell(9, 66, 8, 18), new Cell(8, 66, 7, 18), new Cell(8, 66, 9, 18),
          new Cell(7, 67, 8, 18), new Cell(9, 67, 8, 18), new Cell(8, 67, 7, 18), new Cell(8, 67, 9, 18),
          new Cell(7, 67, 7, 18), new Cell(7, 67, 9, 18), new Cell(9, 67, 7, 18), new Cell(9, 67, 9, 18)),
          new Cell(8, 65, 8, 17), 0,
          List.of(new Point(8, 65, 8), new Point(8, 64, 8), new Point(8, 66, 8), new Point(7, 65, 8),
              new Point(8, 65, 7), new Point(8, 66, 7), new Point(8, 67, 8))),
      new Case("water-remove", List.of(
          new Cell(7, 63, 7, 1), new Cell(8, 63, 7, 1), new Cell(9, 63, 7, 1),
          new Cell(7, 63, 8, 1), new Cell(8, 63, 8, 1), new Cell(9, 63, 8, 1),
          new Cell(7, 63, 9, 1), new Cell(8, 63, 9, 1), new Cell(9, 63, 9, 1),
          new Cell(8, 64, 8, 9)),
          new Cell(8, 64, 8, 9), 0,
          List.of(new Point(8, 64, 8), new Point(8, 65, 8), new Point(7, 64, 8), new Point(9, 64, 8),
              new Point(8, 64, 7), new Point(8, 64, 9))),
      new Case("water-place", List.of(
          new Cell(7, 63, 7, 1), new Cell(8, 63, 7, 1), new Cell(9, 63, 7, 1),
          new Cell(7, 63, 8, 1), new Cell(8, 63, 8, 1), new Cell(9, 63, 8, 1),
          new Cell(7, 63, 9, 1), new Cell(8, 63, 9, 1), new Cell(9, 63, 9, 1)),
          new Cell(8, 64, 8, 0), 9,
          List.of(new Point(8, 64, 8), new Point(8, 65, 8), new Point(7, 64, 8), new Point(9, 64, 8),
              new Point(8, 64, 7), new Point(8, 64, 9)))
    };
  }

  public static void main(String[] args) throws Exception {
    if (args.length != 1) throw new IOException("usage: LightMaterialEditOracle CLIENT_JAR");
    java.nio.file.Path jar = java.nio.file.Path.of(args[0]).toRealPath();
    if (!LightUpdateOracle.sha(jar).equals(LightUpdateOracle.JAR_SHA))
      throw new IOException("wrong client JAR SHA-256");
    Class<?>[] nativeClasses = {fd.class, lm.class, wi.class, uu.class, eb.class, st.class};
    for (Class<?> type : nativeClasses) {
      java.nio.file.Path source = java.nio.file.Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath();
      if (!source.equals(jar)) throw new IOException("native class not loaded from configured client JAR: " + type.getName());
    }
    byte[] input = System.in.readNBytes(REQUEST.length() + 1);
    if (!java.util.Arrays.equals(input, REQUEST.getBytes(StandardCharsets.US_ASCII)) || System.in.read() != -1)
      throw new IOException("request must be exactly 'material-edits-v1\\n'");

    StringBuilder out = new StringBuilder();
    PrintStream realOut = System.out;
    ByteArrayOutputStream bootstrap = new ByteArrayOutputStream();
    System.setOut(new PrintStream(bootstrap, true, StandardCharsets.UTF_8));
    try {
      for (Case scenario : cases()) {
        LightUpdateOracle.W world = LightUpdateOracle.world();
        for (Cell seed : scenario.seed) put(world, seed);
        String beforeDigest = LightUpdateOracle.stateDigest(world);
        out.append("case\t").append(scenario.name).append("\ttarget=")
            .append(scenario.edit.x).append(',').append(scenario.edit.y).append(',').append(scenario.edit.z)
            .append("\treplacement=").append(scenario.replacement)
            .append("\tbefore_digest=").append(beforeDigest).append('\n');
        for (Point point : scenario.samples)
          out.append(LightUpdateOracle.row(world, scenario.name, "before", point.x, point.y, point.z)).append('\n');

        if (!world.a(scenario.edit.x, scenario.edit.y, scenario.edit.z, scenario.replacement, 0))
          throw new IOException("native edit failed for " + scenario.name);
        LightUpdateOracle.settle(world, scenario.edit.x, scenario.edit.y, scenario.edit.z);
        if (world.a(scenario.edit.x, scenario.edit.y, scenario.edit.z) != scenario.replacement)
          throw new IOException("native edit did not publish replacement for " + scenario.name);
        out.append("final\t").append(scenario.name).append('\t')
            .append(LightUpdateOracle.stateDigest(world)).append('\n');
        for (Point point : scenario.samples)
          out.append(LightUpdateOracle.row(world, scenario.name, "after", point.x, point.y, point.z)).append('\n');
      }
    } finally {
      System.setOut(realOut);
    }
    String bootstrapSha = LightUpdateOracle.shaBytes(bootstrap.toByteArray());
    if (!bootstrapSha.equals("0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e"))
      throw new IOException("unexpected native registry bootstrap bytes: " + bootstrapSha);
    System.out.write(out.toString().getBytes(StandardCharsets.US_ASCII));
    System.out.flush();
  }
}
