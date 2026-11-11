/** Capture the exact tessellator calls emitted by native bb's overlay. */
public final class nw {
    public static final nw a = new nw();
    private int color, n; private final double[] xs = new double[4], ys = new double[4];
    public void b() { n = 0; }
    public void b(int value) { color = value; }
    public void a(double x, double y, double z) { xs[n] = x; ys[n++] = y; }
    public void a() {
        if (n != 4) throw new IllegalStateException("expected quad");
        System.out.printf("\t%.0f,%.0f,%.0f,%.0f,%06x", xs[0], ys[0], xs[2]-xs[0], ys[2]-ys[0], color);
    }
}
