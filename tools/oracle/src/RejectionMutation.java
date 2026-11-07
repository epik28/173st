import java.util.Random;

/** Explicitly synthetic broken bounded sampler; never the parity reference. */
public final class RejectionMutation {
    private static final class WithoutRejection extends Random {
        WithoutRejection(long seed) { super(seed); }
        @Override public int nextInt(int bound) { return next(31) % bound; }
    }
    public static void main(String[] args) {
        Random correct = new Random(0);
        Random mutated = new WithoutRejection(0);
        for (int index = 0; index < 2048; index++) {
            System.out.println(index + "\t" + correct.nextInt(1073741825)
                    + "\t" + mutated.nextInt(1073741825));
        }
    }
}
