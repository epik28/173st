import java.nio.file.Files;
import java.nio.file.Path;

/** Native hoe use on disposable in-memory chunks, no saves or GL. */
public final class HoeUseOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 1 || !PlacementOracle.sha(Files.readAllBytes(Path.of(args[0]))).equals(FoodOracle.SHA)) throw new IllegalArgumentException("wrong client JAR");
    java.io.PrintStream out=System.out; System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream()));
    Class.forName("jl"); Class.forName("uu");
    StringBuilder rows=new StringBuilder("item\ttarget\tabove\tside\thandled\tresult_id\tresult_meta\twear\n");
    for(int item=290;item<=294;item++) for(int target:new int[]{1,2,3,12,60})
      for(int above:new int[]{0,1,9,78}) for(int side=0;side<6;side++) {
        PlacementOracle.Provider provider=new PlacementOracle.Provider(); fd world=PlacementOracle.world(provider);
        PlacementOracle.put(world,0,10,0,target,0); if(above!=0) PlacementOracle.put(world,0,11,0,above,0);
        FoodOracle.Player player=(FoodOracle.Player)PlacementOracle.alloc(FoodOracle.Player.class); player.c=new ix(player);
        iz stack=new iz(item,1,0);
        boolean handled=stack.a(player,world,0,10,0,side);
        String[] state=PlacementOracle.state(provider,0,10,0).split(":");
        rows.append(String.format("%d\t%d\t%d\t%d\t%d\t%s\t%s\t%d%n",item,target,above,side,handled?1:0,state[0],state[1],stack.h()));
      }
    System.setOut(out); System.out.print(rows);
  }
}
