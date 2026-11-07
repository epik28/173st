import java.io.*;
import java.lang.reflect.*;
import java.net.*;
import java.nio.charset.StandardCharsets;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;
import java.util.jar.*;

/** CHUNK-001: original Beta nibble calls plus explicitly separate API guards. */
public final class ChunkOracle {
    static final String JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c";
    static long constructed, nativeGets, nativeSets, rawReads;
    static String hash(byte[] bytes) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }
    static final class Native implements AutoCloseable {
        final URLClassLoader loader;
        final Constructor<?> constructor;
        final Method get, set;
        final Field raw;
        final String classHash;
        Native(Path jar) throws Exception {
            if (!hash(Files.readAllBytes(jar)).equals(JAR_SHA256)) throw new IOException("unrecognized client JAR SHA-256");
            try (JarFile file = new JarFile(jar.toFile())) {
                classHash = hash(file.getInputStream(file.getJarEntry("wi.class")).readAllBytes());
            }
            loader = new URLClassLoader(new URL[]{jar.toUri().toURL()}, ClassLoader.getPlatformClassLoader());
            Class<?> type = Class.forName("wi", true, loader);
            if (!Path.of(type.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar.toRealPath())) {
                throw new IOException("original class loaded from unexpected location");
            }
            constructor = type.getConstructor(int.class);
            get = type.getMethod("a", int.class, int.class, int.class);
            set = type.getMethod("a", int.class, int.class, int.class, int.class);
            raw = type.getField("a");
        }
        Object create() throws Exception { constructed++; return constructor.newInstance(32768); }
        byte[] bytes(Object value) throws Exception { rawReads++; return (byte[]) raw.get(value); }
        int get(Object value, int x, int y, int z) throws Exception {
            nativeGets++; return (Integer) get.invoke(value, x, y, z);
        }
        void set(Object value, int x, int y, int z, int nibble) throws Exception {
            nativeSets++; set.invoke(value, x, y, z, nibble);
        }
        String metadata() {
            return "{\"reference_side\":\"Beta 1.7.3 client nibble class\",\"jar_sha256\":\"" + JAR_SHA256
                + "\",\"class\":\"wi\",\"class_sha256\":\"" + classHash
                + "\",\"constructor\":\"wi(int)\",\"get\":\"a(int,int,int)\",\"set\":\"a(int,int,int,int)\",\"raw_field\":\"a:byte[]\","
                + "\"wrapper_scope\":\"coord arithmetic, block indexing/IDs, zero initialization and API validation are our wrapper; nibble allocation/get/set/raw are original\","
                + "\"native_constructors\":" + constructed + ",\"native_gets\":" + nativeGets
                + ",\"native_sets\":" + nativeSets + ",\"raw_reads\":" + rawReads + "}";
        }
        public void close() throws IOException { loader.close(); }
    }
    static int decimal(String value, boolean signed) {
        if (!value.matches(signed ? "[+-]?[0-9]+" : "[+]?[0-9]+")) throw new IllegalArgumentException("invalid decimal");
        return Integer.parseInt(value);
    }
    static int u8(String value) {
        int number = decimal(value, false);
        if (number > 255) throw new IllegalArgumentException("u8 argument outside range");
        return number;
    }
    static boolean valid(int x, int y, int z) { return x < 16 && y < 128 && z < 16; }
    static int index(int x, int y, int z) { return y + z * 128 + x * 2048; }
    static final class State {
        final Native original;
        final byte[] blocks = new byte[32768];
        final Object[] nibble = new Object[3];
        State(Native original) throws Exception {
            this.original = original;
            for (int i=0; i<3; i++) nibble[i] = original.create();
        }
        String operation(String token) throws Exception {
            String[] fields = token.split(":", -1);
            if (fields.length == 2 && fields[0].equals("coord")) {
                int value = decimal(fields[1], true);
                return (value >> 4) + "," + (value & 15);
            }
            if (fields.length == 3 && fields[0].equals("byte")) {
                byte[] array;
                switch (fields[1]) {
                    case "blocks": array = blocks; break;
                    case "data": array = original.bytes(nibble[0]); break;
                    case "block_light": array = original.bytes(nibble[1]); break;
                    case "sky_light": array = original.bytes(nibble[2]); break;
                    default: throw new IllegalArgumentException("unknown array");
                }
                int position = decimal(fields[2], false);
                if (position >= array.length) throw new IllegalArgumentException("byte index outside range");
                return Integer.toString(array[position] & 255);
            }
            boolean setting = fields[0].equals("set");
            if ((setting && fields.length == 8)
                    || ((fields[0].equals("index") || fields[0].equals("get")) && fields.length == 4)) {
                int x=u8(fields[1]), y=u8(fields[2]), z=u8(fields[3]);
                int[] values = new int[4];
                if (setting) for (int i=0; i<4; i++) values[i] = u8(fields[i+4]);
                if (!valid(x,y,z)) return "invalid_coordinate";
                int position = index(x,y,z);
                if (fields[0].equals("index")) return Integer.toString(position);
                if (!setting) return (blocks[position] & 255) + "," + original.get(nibble[0],x,y,z)
                    + "," + original.get(nibble[1],x,y,z) + "," + original.get(nibble[2],x,y,z);
                if (values[1] > 15 || values[2] > 15 || values[3] > 15) return "invalid_nibble";
                blocks[position] = (byte) values[0];
                for (int i=0; i<3; i++) original.set(nibble[i],x,y,z,values[i+1]);
                return "ok";
            }
            throw new IllegalArgumentException("unknown/invalid operation: " + token);
        }
    }
    public static void main(String[] args) {
        try {
            boolean describe = args.length == 2 && args[0].equals("--describe");
            if (args.length < 1 || args.length > 2) throw new IllegalArgumentException("usage: ChunkOracle JAR [AUDIT_JSON] | --describe JAR");
            Path jar = Path.of(args[describe ? 1 : 0]).toRealPath();
            try (Native original = new Native(jar)) {
                if (describe) {
                    Object test = original.create();
                    original.set(test,0,0,0,3); original.set(test,0,1,0,12);
                    if (original.get(test,0,0,0)!=3 || original.get(test,0,1,0)!=12
                        || (original.bytes(test)[0]&255)!=195) throw new IOException("original reflection self-test failed");
                    System.out.println(original.metadata());
                    return;
                }
                BufferedReader input = new BufferedReader(new InputStreamReader(System.in,StandardCharsets.UTF_8));
                PrintWriter output = new PrintWriter(System.out,false,StandardCharsets.UTF_8);
                Set<String> ids = new HashSet<>();
                String line; int total=0;
                while ((line=input.readLine())!=null) {
                    if (line.isEmpty() || line.startsWith("#")) continue;
                    String[] fields=line.split("\t",-1);
                    if (fields.length!=2 || !fields[0].matches("[A-Za-z0-9_-]+") || !ids.add(fields[0])) {
                        throw new IllegalArgumentException("invalid fields/ID or duplicate ID");
                    }
                    String[] ops=fields[1].split(";",-1);
                    total+=ops.length;
                    if (total>100000) throw new IllegalArgumentException("operation limit 100000");
                    State state = new State(original);
                    for (int i=0; i<ops.length; i++) {
                        String value=state.operation(ops[i]);
                        output.print(fields[0]+"\t"+i+"\t"+ops[i].split(":",-1)[0]+"\t"+value+"\n");
                    }
                }
                output.flush();
                if (output.checkError()) throw new IOException("cannot write stdout");
                if (args.length==2) Files.writeString(Path.of(args[1]),original.metadata()+"\n");
            }
        } catch (Exception error) {
            System.err.println("ChunkOracle: " + error);
            System.exit(1);
        }
    }
}
