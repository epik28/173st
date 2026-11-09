import java.awt.image.BufferedImage;
import java.io.BufferedReader;
import java.io.InputStream;
import java.io.InputStreamReader;
import java.lang.reflect.Field;
import java.nio.charset.StandardCharsets;
import java.util.Locale;
import javax.imageio.ImageIO;

/** Calls the pinned native ScaledResolution constructor and captures font metrics. */
public final class MenuParityOracle {
    private static void resolution(String line) throws Exception {
        String[] fields = line.split("\\t", -1);
        if (fields.length != 4 || !fields[0].equals("scale")) {
            throw new IllegalArgumentException("expected scale<TAB>width<TAB>height<TAB>guiScale");
        }
        int width = Integer.parseInt(fields[1]);
        int height = Integer.parseInt(fields[2]);
        int guiScale = Integer.parseInt(fields[3]);
        if (width < 1 || height < 1 || guiScale < 0 || guiScale > 3) {
            throw new IllegalArgumentException("invalid resolution or GUI scale");
        }
        kv options = new kv();
        Field option = kv.class.getField("I");
        option.setInt(options, guiScale);
        qq scaled = new qq(options, width, height);
        System.out.printf(Locale.ROOT, "scale\t%d\t%d\t%d\t%d\t%d\t%d\t%s\t%s%n",
                width, height, guiScale, scaled.c, scaled.a(), scaled.b(),
                Double.toString(scaled.a), Double.toString(scaled.b));
    }

    /* sj's pinned constructor scans each 8x8 glyph cell from x=7 leftward,
       checks the low alpha byte in all eight rows, then stores rightmost+2.
       It special-cases glyph 32 to rightmost=2. This emits that table without
       constructing sj, whose constructor also requires an OpenGL context. */
    private static void fontMetrics() throws Exception {
        BufferedImage image;
        try (InputStream stream = MenuParityOracle.class.getResourceAsStream("/font/default.png")) {
            if (stream == null) throw new IllegalStateException("font/default.png absent from pinned JAR");
            image = ImageIO.read(stream);
        }
        if (image == null || image.getWidth() != 128 || image.getHeight() != 128) {
            throw new IllegalStateException("unexpected native font dimensions");
        }
        int[] pixels = image.getRGB(0, 0, image.getWidth(), image.getHeight(), null, 0, image.getWidth());
        for (int glyph = 0; glyph < 256; glyph++) {
            int x = glyph % 16;
            int y = glyph / 16;
            int rightmost = 7;
            for (; rightmost >= 0; rightmost--) {
                boolean empty = true;
                int px = x * 8 + rightmost;
                for (int row = 0; row < 8; row++) {
                    if ((pixels[(y * 8 + row) * image.getWidth() + px] & 0xff) > 0) {
                        empty = false;
                        break;
                    }
                }
                if (!empty) break;
            }
            if (glyph == 32) rightmost = 2;
            System.out.printf(Locale.ROOT, "glyph\t%d\t%d%n", glyph, rightmost + 2);
        }
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 0) throw new IllegalArgumentException("no arguments accepted");
        try (BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8))) {
            String line;
            while ((line = input.readLine()) != null) {
                if (line.isEmpty()) continue;
                resolution(line);
            }
        }
        fontMetrics();
    }
}
