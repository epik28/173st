import java.io.*;
import java.nio.charset.StandardCharsets;
import java.util.*;

/** RNG-002 reference; every random operation delegates to java.util.Random. */
public final class RandomExtendedOracle {
    static long integer(String value, boolean wide) {
        if (!value.matches("[+-]?[0-9]+")) throw new IllegalArgumentException("invalid decimal");
        return wide ? Long.parseLong(value) : Integer.parseInt(value);
    }

    static String operation(Random rng, String token) {
        String[] parts = token.split(":", -1);
        if (parts.length == 1) {
            switch (token) {
                case "int": return Integer.toString(rng.nextInt());
                case "long": return Long.toString(rng.nextLong());
                case "boolean": return rng.nextBoolean() ? "1" : "0";
                case "float": return String.format(Locale.ROOT, "%08x", Float.floatToRawIntBits(rng.nextFloat()));
                case "double": return String.format(Locale.ROOT, "%016x", Double.doubleToRawLongBits(rng.nextDouble()));
                default: throw new IllegalArgumentException("unknown operation: " + token);
            }
        }
        if (parts.length == 2 && parts[0].equals("seed")) {
            rng.setSeed(integer(parts[1], true));
            return "ok";
        }
        if (parts.length == 2 && parts[0].equals("bound")) {
            int bound = (int) integer(parts[1], false);
            try {
                return Integer.toString(rng.nextInt(bound));
            } catch (IllegalArgumentException expected) {
                return "invalid_bound";
            }
        }
        throw new IllegalArgumentException("invalid operation: " + token);
    }

    public static void main(String[] args) {
        try {
            BufferedReader input = new BufferedReader(new InputStreamReader(System.in, StandardCharsets.UTF_8));
            PrintWriter output = new PrintWriter(System.out, false, StandardCharsets.UTF_8);
            Set<String> ids = new HashSet<>();
            String line;
            int total = 0;
            while ((line = input.readLine()) != null) {
                if (line.isEmpty() || line.startsWith("#")) continue;
                String[] fields = line.split("\t", -1);
                if (fields.length != 3 || !fields[0].matches("[A-Za-z0-9_-]+") || !ids.add(fields[0])) {
                    throw new IllegalArgumentException("invalid fields/ID or duplicate ID");
                }
                Random rng = new Random(integer(fields[1], true));
                String[] operations = fields[2].split(";", -1);
                total += operations.length;
                if (total > 100000) throw new IllegalArgumentException("operation limit 100000");
                for (int index = 0; index < operations.length; index++) {
                    String value = operation(rng, operations[index]);
                    String name = operations[index].split(":", -1)[0];
                    output.print(fields[0] + "\t" + index + "\t" + name + "\t" + value + "\n");
                }
            }
            output.flush();
            if (output.checkError()) throw new IOException("cannot write stdout");
        } catch (Exception error) {
            System.err.println("RandomExtendedOracle: " + error.getMessage());
            System.exit(1);
        }
    }
}
