import java.nio.file.Files;
import java.nio.file.Path;

/** Native block dispatch before selected food use on an in-memory stone block. */
public final class FoodBlockRouteOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !PlacementOracle.sha(Files.readAllBytes(Path.of(args[0]))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("uu");
    PlacementOracle.Provider provider=new PlacementOracle.Provider(); fd world=PlacementOracle.world(provider);
    PlacementOracle.put(world,0,10,0,1,0);
    FoodOracle.Player player=(FoodOracle.Player)PlacementOracle.alloc(FoodOracle.Player.class); player.c=new ix(player); player.Y=10; player.E=20;
    player.c.a[0]=new iz(260,1,0); ob controller=(ob)PlacementOracle.alloc(ob.class);
    boolean block=controller.a(player,world,player.c.a[0],0,10,0,1);
    boolean item=!block && controller.a(player,world,player.c.a[0]);
    System.setOut(out);
    System.out.println("target\titem\tblock_handled\titem_handled\thealth\tslot_empty");
    System.out.printf("1\t260\t%d\t%d\t%d\t%d%n",block?1:0,item?1:0,player.Y,player.c.a[0]==null?1:0);
  }
}
