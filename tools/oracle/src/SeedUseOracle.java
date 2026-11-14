import java.nio.file.Files;
import java.nio.file.Path;

/** Original wheat-seed use on isolated blocks; no save access. */
public final class SeedUseOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !PlacementOracle.sha(Files.readAllBytes(Path.of(args[0]))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("uu");
    StringBuilder rows=new StringBuilder("target\tmetadata\tabove\tside\thandled\ttarget_after\tabove_after\tcount_after\n");
    for(int target:new int[]{1,2,3,12,60}) for(int meta:new int[]{0,1,7,15})
      for(int above:new int[]{0,1,9,31,78}) for(int side=0;side<6;side++) {
        PlacementOracle.Provider provider=new PlacementOracle.Provider(); fd world=PlacementOracle.world(provider);
        PlacementOracle.put(world,0,10,0,target,meta); if(above!=0) PlacementOracle.put(world,0,11,0,above,0);
        FoodOracle.Player player=(FoodOracle.Player)PlacementOracle.alloc(FoodOracle.Player.class); player.c=new ix(player);
        iz stack=new iz(295,4,0); boolean handled=stack.a(player,world,0,10,0,side);
        rows.append(String.format("%d\t%d\t%d\t%d\t%d\t%s\t%s\t%d%n",target,meta,above,side,handled?1:0,
          PlacementOracle.state(provider,0,10,0),PlacementOracle.state(provider,0,11,0),stack.a));
      }
    System.setOut(out); System.out.print(rows);
  }
}
