import java.lang.reflect.Field;
import java.net.URL;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;

/** Queries the pinned client item registry without creating a world or window. */
public final class InventoryUiOracle {
    private static String sha(byte[] bytes) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }

    private static String source(Class<?> type) throws Exception {
        URL url = type.getProtectionDomain().getCodeSource().getLocation();
        return Path.of(url.toURI()).toRealPath().toString();
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("usage: InventoryUiOracle <pinned-client.jar>");
        Path jar = Path.of(args[0]).toRealPath();
        byte[] jarBytes = java.nio.file.Files.readAllBytes(jar);
        if (!sha(jarBytes).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))
            throw new IllegalStateException("wrong pinned client JAR");
        // The original item registry depends on block initialization order.
        java.io.PrintStream originalOut = System.out;
        try {
            System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
            Class.forName("uu");
        } finally {
            System.setOut(originalOut);
        }
        Field icon = gm.class.getDeclaredField("bh");
        icon.setAccessible(true);
        System.out.println("jar_sha256\t" + sha(jarBytes));
        System.out.println("code_source\t" + source(iz.class));
        for (int id : new int[] {1, 3, 332, 267}) {
            iz stack = new iz(id, 1, 0);
            gm item = stack.a();
            if (item == null) throw new IllegalStateException("native item registry missing ID " + id);
            System.out.println("icon\t" + id + "\t" + item.getClass().getName() + "\t" + icon.getInt(item));
        }
    }
}
