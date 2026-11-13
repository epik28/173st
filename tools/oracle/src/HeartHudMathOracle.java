import java.util.Random;
/** Reproduces branches of pinned uq.a(FZII)V; does not run native GUI/GL. */
public final class HeartHudMathOracle {
  public static void main(String[] args) {
    System.out.println("ticks\thealth\tprevious\thurt_timer\tindex\ty_offset\tbackground_uv\tprevious_uv\tcurrent_uv");
    for (int ticks : new int[]{0, 1, 11, 100, 100000, Integer.MAX_VALUE})
      for (int health : new int[]{0, 1, 4, 5, 19, 20})
        for (int previous : new int[]{0, 7, 20})
          for (int timer : new int[]{0, 9, 10, 11, 12, 14, 15, 18, 20}) {
            boolean flash = timer / 3 % 2 == 1 && timer >= 10;
            Random random = new Random((long)(ticks * 312871));
            for (int i=0; i<10; i++) {
              int offset = health <= 4 ? random.nextInt(2) : 0;
              int threshold = 2*i+1;
              int previousUv = !flash ? -1 : threshold < previous ? 70 : threshold == previous ? 79 : -1;
              int currentUv = threshold < health ? 52 : threshold == health ? 61 : -1;
              System.out.printf("%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%d%n", ticks, health, previous, timer, i, offset, flash ? 25 : 16, previousUv, currentUv);
            }
          }
  }
}
