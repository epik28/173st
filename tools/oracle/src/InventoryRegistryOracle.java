import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import java.util.zip.ZipFile;

/** Query item icons and stack limits from the pinned Beta 1.7.3 registry. */
public final class InventoryRegistryOracle {
    private static String hash(byte[] data) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(data));
    }
    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("expected client JAR path");
        Path jar = Path.of(args[0]).toRealPath();
        if (!hash(Files.readAllBytes(jar)).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))
            throw new IllegalStateException("wrong client JAR");
        String[][] classes = {
            {"iz", "3e8a07bf8c43729c8367bab454af876e698505620f230e012432c04e3828d6d6"},
            {"gm", "f9d971a83efa0c6399f1fc4bc0bc06c6a6249908afc6b46f5889a49365c190c7"},
            {"uu", "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845"},
        };
        try (ZipFile zip = new ZipFile(jar.toFile())) {
            for (String[] cls : classes) {
                if (!Path.of(Class.forName(cls[0], false, InventoryRegistryOracle.class.getClassLoader())
                        .getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))
                    throw new IllegalStateException("class loaded from wrong JAR: " + cls[0]);
                if (!hash(zip.getInputStream(zip.getEntry(cls[0] + ".class")).readAllBytes()).equals(cls[1]))
                    throw new IllegalStateException("wrong class: " + cls[0]);
            }
        }
        var out = System.out;
        try {
            System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
            Class.forName("uu");
        } finally { System.setOut(out); }
        System.out.println("id\ticon0\ticon1\ticon15\tmax_stack");
        for (int id = 256; id <= 359; id++) {
            iz stack = new iz(id, 1, 0);
            if (stack.a() == null) throw new IllegalStateException("missing item " + id);
            System.out.printf("%d\t%d\t%d\t%d\t%d%n", id, stack.b(),
                    new iz(id, 1, 1).b(), new iz(id, 1, 15).b(), stack.c());
        }
    }
}
