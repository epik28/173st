import java.io.*;
import java.lang.reflect.Field;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Calls the ORIGINAL Beta qj reader; only TSV, path, size and metadata are ours. */
public final class RegionOracle {
    private static final long LIMIT = 64L * 1024 * 1024;
    private static final String JAR_HASH = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";

    private static String hash(byte[] data) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(data));
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 4) throw new IllegalArgumentException("RegionOracle JAR INPUT_COPY_DIR PAYLOAD_DIR AUDIT_JSON");
        Path jar = Path.of(args[0]).toRealPath();
        if (!hash(Files.readAllBytes(jar)).equals(JAR_HASH)) throw new IOException("wrong original JAR hash");
        if (!Path.of(qj.class.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))
            throw new IOException("qj was not loaded from the supplied original JAR");
        Path input = Path.of(args[1]).toRealPath();
        if (!Files.readString(input.resolve(".region-input-copy")).equals("REGION-001 copied inputs\n"))
            throw new IOException("requires snapshot copy marker");
        Path output = Path.of(args[2]).toAbsolutePath().normalize();
        Files.createDirectories(output);
        output = output.toRealPath();
        if (output.startsWith(input) || input.startsWith(output)) throw new IOException("payload/input directories overlap");
        Map<Path, qj> opened = new LinkedHashMap<>();
        Map<Path, String> hashes = new LinkedHashMap<>();
        Set<String> ids = new HashSet<>();
        Field locations = qj.class.getDeclaredField("d");
        Field timestamps = qj.class.getDeclaredField("e");
        locations.setAccessible(true);
        timestamps.setAccessible(true);
        int calls = 0, present = 0, absent = 0;
        try {
            BufferedReader reader = new BufferedReader(new InputStreamReader(System.in, java.nio.charset.StandardCharsets.UTF_8));
            String line;
            while ((line = reader.readLine()) != null) {
                if (line.isEmpty() || line.startsWith("#")) continue;
                String[] f = line.split("\t", -1);
                if (f.length != 4 || !f[0].matches("[A-Za-z0-9_-]+") || !ids.add(f[0]))
                    throw new IOException("invalid fields/ID");
                if (ids.size() > 100000) throw new IOException("case limit");
                if (!f[2].matches("-?(0|[1-9][0-9]*)") || !f[3].matches("-?(0|[1-9][0-9]*)"))
                    throw new IOException("invalid coordinate syntax");
                int cx = Integer.parseInt(f[2]), cz = Integer.parseInt(f[3]);
                String name = "r." + Math.floorDiv(cx, 32) + "." + Math.floorDiv(cz, 32) + ".mcr";
                if (!f[1].equals(name)) throw new IOException("region basename does not match coordinates");
                Path path = input.resolve(name);
                if (Files.isSymbolicLink(path) || !path.toRealPath().getParent().equals(input))
                    throw new IOException("region path escapes input copy");
                qj region = opened.get(path);
                if (region == null) {
                    long size = Files.size(path);
                    // qj's constructor opens rw and repairs tiny/unaligned files.
                    // These guards prevent any native repair, even on copies.
                    if (size < 8192 || size % 4096 != 0 || size > 256L*1024*1024)
                        throw new IOException("wrapper region size/alignment guard");
                    hashes.put(path, hash(Files.readAllBytes(path)));
                    region = new qj(path.toFile());
                    opened.put(path, region);
                }
                int x = Math.floorMod(cx, 32), z = Math.floorMod(cz, 32), slot = x + z * 32;
                int location = ((int[]) locations.get(region))[slot];
                long timestamp = Integer.toUnsignedLong(((int[]) timestamps.get(region))[slot]);
                calls++;
                try (DataInputStream nativeStream = region.a(x, z)) {
                    if (nativeStream == null) {
                        if (location != 0) throw new IOException("native reader returned null for occupied slot");
                        System.out.println(f[0] + "\tabsent\t-\t-\t-\t-");
                        absent++;
                        continue;
                    }
                    int compression;
                    try (RandomAccessFile metadata = new RandomAccessFile(path.toFile(), "r")) {
                        metadata.seek((long)(location >>> 8)*4096 + 4);
                        compression = metadata.readUnsignedByte();
                    }
                    ByteArrayOutputStream payload = new ByteArrayOutputStream();
                    byte[] buffer = new byte[8192];
                    int length;
                    while ((length = nativeStream.read(buffer)) != -1) {
                        if ((long)payload.size() + length > LIMIT) throw new IOException("decompressed limit exceeded");
                        payload.write(buffer, 0, length);
                    }
                    byte[] bytes = payload.toByteArray();
                    Files.write(output.resolve(f[0] + ".nbt"), bytes, StandardOpenOption.CREATE_NEW);
                    System.out.println(f[0] + "\tpresent\t" + timestamp + "\t" + compression + "\t" + bytes.length + "\t" + hash(bytes));
                    present++;
                }
            }
        } finally {
            for (qj region : opened.values()) region.b();
            for (var entry : hashes.entrySet())
                if (!entry.getValue().equals(hash(Files.readAllBytes(entry.getKey()))))
                    throw new IOException("input copy was modified: " + entry.getKey());
        }
        String classHash;
        try (InputStream bytes = qj.class.getResourceAsStream("/qj.class")) { classHash = hash(bytes.readAllBytes()); }
        String audit = "{\"class\":\"qj\",\"method\":\"a(int,int)\",\"jar_sha256\":\"" + JAR_HASH
            + "\",\"class_sha256\":\"" + classHash + "\",\"constructors\":" + opened.size()
            + ",\"native_reads\":" + calls + ",\"present\":" + present + ",\"absent\":" + absent + "}\n";
        Files.writeString(Path.of(args[3]), audit, StandardOpenOption.CREATE_NEW);
        System.err.print(audit);
    }
}
