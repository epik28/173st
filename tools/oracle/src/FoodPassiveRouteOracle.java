import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;

/** Native block interaction followed by food use for ordinary target blocks. */
public final class FoodPassiveRouteOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !PlacementOracle.sha(Files.readAllBytes(Path.of(args[0]))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("uu");
    int[] targets={1,2,3,4,5,12,13,17,18,20,24,31,35,44,49,78,81,82,86,87,88,89,91};
    StringBuilder rows=new StringBuilder("target\tside\tblock_handled\titem_handled\thealth\tslot_empty\n");
    for(int id:targets) for(int side=0;side<6;side++) {
      PlacementOracle.Provider provider=new PlacementOracle.Provider(); fd world=PlacementOracle.world(provider);
      PlacementOracle.put(world,0,10,0,id,0);
      FoodOracle.Player player=(FoodOracle.Player)PlacementOracle.alloc(FoodOracle.Player.class); player.c=new ix(player); player.Y=10; player.E=20;
      player.c.a[0]=new iz(260,1,0); ob controller=(ob)PlacementOracle.alloc(ob.class);
      boolean block=controller.a(player,world,player.c.a[0],0,10,0,side);
      boolean item=!block && controller.a(player,world,player.c.a[0]);
      rows.append(String.format("%d\t%d\t%d\t%d\t%d\t%d%n",id,side,block?1:0,item?1:0,player.Y,player.c.a[0]==null?1:0));
    }
    System.setOut(out); System.out.print(rows);
  }
}
