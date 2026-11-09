import java.io.BufferedReader;
import java.io.ByteArrayOutputStream;
import java.io.InputStreamReader;
import java.io.PrintStream;
import java.lang.reflect.Method;
import java.net.URL;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.ArrayList;
import java.util.HexFormat;
import java.util.List;

/** Bounded, in-memory adapter over Beta 1.7.3 iz (ItemStack) and ix (inventory). */
public final class ItemsOracle {
    private static final List<String> rows = new ArrayList<>();
    private static byte[] bootstrapOutput = new byte[0];
    private static int stackCtor, inventoryCtor, setCalls, splitCalls, removeCalls, addCalls, cycleCalls, selectedReads;

    private static String stack(iz s) {
        return s == null ? "-" : s.c + "," + s.a + "," + s.h() + "," + s.c() + "," + s.a().a();
    }

    private static String slots(ix inv) {
        StringBuilder b = new StringBuilder();
        for (int i = 0; i < inv.a.length; i++) {
            if (i != 0) b.append('|');
            b.append(stack(inv.a[i]));
        }
        return b.toString();
    }

    private static void state(String scenario, String step, String result, ix inv) {
        iz selected = inv.b(); selectedReads++;
        rows.add(scenario + "\t" + step + "\t" + result + "\t" + inv.c + "\t" + stack(selected) + "\t" + slots(inv));
    }

    private static iz item(int id, int count, int damage) {
        stackCtor++;
        return new iz(id, count, damage);
    }

    private static ix inventory() { inventoryCtor++; return new ix(null); }

    private static void scenario(String[] f) {
        String name = f[0];
        switch (name) {
            case "split_remove_replace": {
                int id = Integer.parseInt(f[1]), count = Integer.parseInt(f[2]), split = Integer.parseInt(f[3]);
                ix inv = inventory();
                state(name, "empty", "none", inv);
                setCalls++; inv.a(0, item(id, count, 0));
                state(name, "set", "slot=0", inv);
                splitCalls++; iz part = inv.a(0, split);
                state(name, "split", "returned=" + stack(part), inv);
                removeCalls++; boolean removed = inv.c(id);
                state(name, "remove_one_by_id", "returned=" + removed + ";id=" + id, inv);
                setCalls++; inv.a(0, item(3, 2, 0));
                state(name, "replace", "slot=0", inv);
                break;
            }
            case "merge": {
                int id = Integer.parseInt(f[1]), have = Integer.parseInt(f[2]), incoming = Integer.parseInt(f[3]);
                ix inv = inventory(); setCalls++; inv.a(0, item(id, have, 0));
                iz incomingStack = item(id, incoming, 0);
                state(name, "before", "incoming=" + stack(incomingStack), inv);
                iz input = item(id, incoming, 0); addCalls++; boolean accepted = inv.a(input);
                state(name, "after", "accepted=" + accepted + ";remaining=" + stack(input), inv);
                break;
            }
            case "overflow": {
                int fullId = Integer.parseInt(f[1]), incomingId = Integer.parseInt(f[2]);
                ix inv = inventory();
                for (int i = 0; i < inv.a.length; i++) { setCalls++; inv.a(i, item(fullId, 64, 0)); }
                iz input = item(incomingId, 1, 0); state(name, "before", "incoming=" + stack(input), inv);
                addCalls++; boolean accepted = inv.a(input);
                state(name, "after", "accepted=" + accepted + ";remaining=" + stack(input), inv);
                break;
            }
            case "smaller_limit": {
                int id = Integer.parseInt(f[1]), have = Integer.parseInt(f[2]), incoming = Integer.parseInt(f[3]);
                ix inv = inventory(); setCalls++; inv.a(0, item(id, have, 0));
                iz input = item(id, incoming, 0); state(name, "before", "max=" + input.c() + ";incoming=" + stack(input), inv);
                addCalls++; boolean accepted = inv.a(input);
                state(name, "after", "accepted=" + accepted + ";max=" + input.c() + ";remaining=" + stack(input), inv);
                break;
            }
            case "non_stackable": {
                int id = Integer.parseInt(f[1]);
                ix inv = inventory(); setCalls++; inv.a(0, item(id, 1, 0));
                iz input = item(id, 1, 0); state(name, "before", "max=" + input.c(), inv);
                addCalls++; boolean accepted = inv.a(input);
                state(name, "after", "accepted=" + accepted + ";remaining=" + stack(input), inv);
                break;
            }
            case "hotbar_bounds": {
                ix inv = inventory();
                setCalls++; inv.a(0, item(1, 2, 0));
                setCalls++; inv.a(8, item(3, 4, 0));
                state(name, "initial", "", inv);
                cycleCalls++; inv.b(1); state(name, "cycle_positive", "", inv);
                cycleCalls++; inv.b(-1); state(name, "cycle_negative", "", inv);
                cycleCalls++; for (int i = 0; i < 9; i++) inv.b(1);
                state(name, "cycle_wrap", "", inv);
                inv.c = -1; state(name, "index_minus_one", "", inv);
                inv.c = 9; state(name, "index_nine", "", inv);
                inv.c = 0; inv.a(3, true); state(name, "select_id_3", "", inv);
                inv.a(9999, true); state(name, "select_missing_id", "", inv);
                break;
            }
            default: throw new IllegalArgumentException("unknown scenario: " + name);
        }
    }

    private static String sha(byte[] bytes) throws Exception {
        return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(bytes));
    }

    private static String classHash(Path jar, String name) throws Exception {
        try (var z = new java.util.zip.ZipFile(jar.toFile())) {
            return sha(z.getInputStream(z.getEntry(name.replace('.', '/') + ".class")).readAllBytes());
        }
    }

    private static String source(Class<?> c) throws Exception {
        URL u = c.getProtectionDomain().getCodeSource().getLocation();
        return Path.of(u.toURI()).toRealPath().toString();
    }

    private static String methods(Class<?> c) {
        List<String> found = new ArrayList<>();
        for (Method m : c.getDeclaredMethods()) {
            if ((c == iz.class && List.of("a", "c", "h").contains(m.getName())) ||
                (c == ix.class && List.of("a", "b", "c").contains(m.getName())))
                found.add(m.getName() + java.util.Arrays.toString(m.getParameterTypes()) + ":" + m.getReturnType().getName());
        }
        return found.toString();
    }

    public static void main(String[] args) throws Exception {
        if (args.length != 1) throw new IllegalArgumentException("usage: ItemsOracle <pinned-client.jar>");
        Path jar = Path.of(args[0]).toRealPath();
        if (!sha(Files.readAllBytes(jar)).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))
            throw new IllegalStateException("wrong pinned client JAR");
        // The original static item registry must be initialized before first touching gm through iz.
        PrintStream originalOut = System.out;
        ByteArrayOutputStream capturedBootstrap = new ByteArrayOutputStream();
        try (PrintStream quiet = new PrintStream(capturedBootstrap, true, java.nio.charset.StandardCharsets.UTF_8)) {
            System.setOut(quiet);
            Class.forName("uu");
        } finally {
            System.setOut(originalOut);
        }
        bootstrapOutput = capturedBootstrap.toByteArray();
        BufferedReader input = new BufferedReader(new InputStreamReader(System.in));
        for (String line; (line = input.readLine()) != null;) {
            if (line.isBlank() || line.startsWith("#")) continue;
            scenario(line.split("\\t", -1));
        }
        for (String row : rows) System.out.println(row);
        System.err.println("{\"jar_sha256\":\"" + sha(Files.readAllBytes(jar)) + "\",\"code_source\":\"" + source(iz.class) +
            "\",\"classes\":{\"iz\":\"" + classHash(jar,"iz") + "\",\"ix\":\"" + classHash(jar,"ix") +
            "\",\"gm\":\"" + classHash(jar,"gm") + "\",\"uu\":\"" + classHash(jar,"uu") + "\",\"gs\":\"" + classHash(jar,"gs") +
            "\",\"jl\":\"" + classHash(jar,"jl") + "\"},\"bootstrap\":\"Class.forName(uu) before iz/gm registry access; no World/player/window/save\",\"bootstrap_stdout_sha256\":\"" + sha(bootstrapOutput) + "\",\"native_methods\":{\"stack_ctor\":\"iz(int,int,int)\",\"stack_split\":\"iz.a(int)\",\"stack_limit\":\"iz.c()\",\"stack_damage\":\"iz.h()\",\"inventory_ctor\":\"ix(gs) with null owner\",\"slot_set\":\"ix.a(int,iz)\",\"slot_split_or_remove\":\"ix.a(int,int)\",\"item_id_remove\":\"ix.c(int)\",\"insert_merge\":\"ix.a(iz)\",\"hotbar_cycle\":\"ix.b(int)\",\"selected_slot_lookup\":\"ix.b()\",\"select_item_id\":\"ix.a(int,boolean)\"},\"direct_calls\":{\"stack_ctor\":" + stackCtor +
            ",\"inventory_ctor\":" + inventoryCtor + ",\"slot_set\":" + setCalls + ",\"slot_split\":" + splitCalls +
            ",\"item_id_remove\":" + removeCalls + ",\"insert_merge\":" + addCalls + ",\"hotbar_cycle\":" + cycleCalls +
            ",\"selected_slot_lookup\":" + selectedReads + "},\"rows\":" + rows.size() + "}");
    }
}
