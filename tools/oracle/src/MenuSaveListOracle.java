import java.io.File;
import java.util.List;
import java.util.Collections;
import java.util.Locale;

/** Invokes the pinned native save metadata reader without constructing a world. */
public final class MenuSaveListOracle {
    public static void main(String[] args) {
        if (args.length != 1) throw new IllegalArgumentException("expected disposable saves directory");
        List worlds = new hi(new File(args[0])).b();
        Collections.sort(worlds);
        for (Object value : worlds) {
            vb world = (vb) value;
            System.out.printf(Locale.ROOT, "world\t%s\t%s\t%d\t%d\t%b%n",
                    world.a(), world.b(), world.e(), world.c(), world.d());
        }
    }
}
