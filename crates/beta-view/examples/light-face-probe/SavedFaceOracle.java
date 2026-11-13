import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.util.*;

/** Native faces in a small real-world neighborhood, editing only an isolated temporary copy. */
public class SavedFaceOracle {
  static final int[][] D={{0,-1,0},{0,1,0},{0,0,-1},{0,0,1},{-1,0,0},{1,0,0}};
  public static void main(String[] args)throws Exception {
    Path jar=Path.of(args[0]).toRealPath(), dir=Path.of(args[1]).toRealPath();
    if(!LightUpdateOracle.sha(jar).equals(LightUpdateOracle.JAR_SHA)) throw new IOException("wrong client JAR");
    if(!dir.getFileName().toString().startsWith("saved-face-probe-")) throw new IOException("requires an isolated probe temp copy");
    for(Class<?> c:List.of(fd.class,lm.class,wi.class,uu.class,eb.class,st.class,ij.class,nu.class,qj.class,cv.class,nw.class))
      if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IOException("unexpected native class origin "+c);
    PrintStream output=System.out; System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    LightUpdateOracle.W world=LightUpdateOracle.world(); Map<String,qj> regions=new HashMap<>();
    try { for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++) {
      String name="r."+Math.floorDiv(cx,32)+"."+Math.floorDiv(cz,32)+".mcr";
      qj region=regions.get(name); if(region==null){region=new qj(dir.resolve(name).toFile());regions.put(name,region);}
      try(DataInputStream input=region.a(Math.floorMod(cx,32),Math.floorMod(cz,32))){
        if(input==null) throw new IOException("missing chunk "+cx+","+cz);
        ij raw=ij.b(input); if(!(raw instanceof nu))throw new IOException("invalid NBT"); nu tag=((nu)raw).k("Level");
        if(tag.e("xPos")!=cx||tag.e("zPos")!=cz)throw new IOException("chunk mismatch");
        byte[] blocks=tag.j("Blocks"),meta=tag.j("Data"),sky=tag.j("SkyLight"),light=tag.j("BlockLight"),height=tag.j("HeightMap");
        if(blocks.length!=32768||meta.length!=16384||sky.length!=16384||light.length!=16384||height.length!=256)throw new IOException("invalid array sizes");
        lm chunk=world.c(cx,cz);chunk.b=blocks;chunk.e=new wi(meta);chunk.f=new wi(sky);chunk.g=new wi(light);chunk.h=height;
      }
    }} finally {for(qj region:regions.values())region.b();}
    if(world.a(-5,66,4)!=2||world.a(-5,67,4)!=0)throw new IOException("source does not match logged edit");
    cv renderer=new cv(world);cv.a=false;nw tess=nw.a;
    Field data=nw.class.getDeclaredField("g"),offset=nw.class.getDeclaredField("o");data.setAccessible(true);offset.setAccessible(true);int[] a=(int[])data.get(tess);tess.b();
    StringBuilder result=new StringBuilder();
    for(int phase=0;phase<2;phase++) {
      if(phase==1)LightMaterialEditV2Oracle.put(world,new LightMaterialEditV2Oracle.Cell(-5,67,4,1,0));
      for(int x=-7;x<=-3;x++)for(int y=65;y<=68;y++)for(int z=2;z<=6;z++) {
        int id=world.a(x,y,z);if(id!=1 && id!=2)continue;
        result.append("case\t").append(phase).append('\t').append(x).append('\t').append(y).append('\t').append(z).append('\t').append(id);
        for(int[] d:D) {int nx=x+d[0],ny=y+d[1],nz=z+d[2];lm c=world.c(nx>>4,nz>>4);if(c==null||ny<0||ny>=128)throw new IOException("unloaded face neighbor");
          result.append('\t').append(world.g(nx,ny,nz)?1:0).append(',').append(LightUpdateOracle.sky(c,nx,ny,nz)).append(',').append(LightUpdateOracle.block(c,nx,ny,nz));}
        result.append('\n');
        offset.setInt(tess,0);renderer.b(uu.m[id],x,y,z,1f,1f,1f);int end=offset.getInt(tess);
        if(end%48!=0)throw new IOException("unexpected quad layout");
        for(int i=0;i<end;i+=48)result.append("face\t").append(phase).append('\t').append(x).append('\t').append(y).append('\t').append(z).append('\t').append(LightFaceOracle.face(a,i,x,y,z)).append('\t').append(String.format(Locale.ROOT,"%08x",a[i+5])).append('\n');
      }
    }
    System.setOut(output);System.out.print(result);
  }
}
