import java.io.BufferedReader;
import java.io.InputStreamReader;
import java.io.PrintWriter;
import java.nio.charset.StandardCharsets;
import java.util.HashSet;
import java.util.Random;
import java.util.Set;

/** Independent RNG-001 reference: delegates all random values to the Java API. */
public final class RandomOracle {
    public static void main(String[] args) {
        try {
            run();
        } catch (Exception error) {
            System.err.println("RandomOracle: " + error.getMessage());
            System.exit(1);
        }
    }

    private static void run() throws Exception {
        BufferedReader input = new BufferedReader(
                new InputStreamReader(System.in, StandardCharsets.UTF_8));
        PrintWriter output = new PrintWriter(System.out, false, StandardCharsets.UTF_8);
        Set<String> ids = new HashSet<>();
        String line;
        int lineNumber = 0;
        while ((line = input.readLine()) != null) {
            lineNumber++;
            if (line.isEmpty() || line.startsWith("#")) {
                continue;
            }
            String[] fields = line.split("\t", -1);
            if (fields.length != 3 || !fields[0].matches("[A-Za-z0-9_-]+")) {
                throw new IllegalArgumentException("line " + lineNumber + ": invalid fields/ID");
            }
            if (!ids.add(fields[0])) {
                throw new IllegalArgumentException("line " + lineNumber + ": duplicate ID");
            }
            if (!fields[1].matches("[+-]?[0-9]+") || !fields[2].matches("[+]?[0-9]+")) {
                throw new IllegalArgumentException("line " + lineNumber + ": invalid decimal");
            }
            long seed = Long.parseLong(fields[1]);
            int count = Integer.parseInt(fields[2]);
            if (count < 1 || count > 100000) {
                throw new IllegalArgumentException("line " + lineNumber + ": count outside 1..100000");
            }
            Random random = new Random(seed);
            for (int index = 0; index < count; index++) {
                output.print(fields[0] + "\t" + index + "\t" + random.nextInt() + "\n");
            }
        }
        output.flush();
        if (output.checkError()) {
            throw new IllegalStateException("cannot write output");
        }
    }
}
