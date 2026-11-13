import java.io.*;
import java.nio.file.*;
import java.util.*;

/** Read chunk NBT from a disposable copied region set, then edit only in memory. */
public final class LightSavedWorldOracle {
  public static void main(String[] args) throws Exception {
    if (args.length != 2) throw new IOException("usage: CLIENT_JAR COPIED_REGION_DIR");
    Path jar=Path.of(args[0]).toRealPath(), input=Path.of(args[1]).toRealPath();
    if (!LightUpdateOracle.sha(jar).equals(LightUpdateOracle.JAR_SHA)) throw new IOException("wrong client JAR");
    if (!input.toString().contains("/artifacts/")) throw new IOException("input must be a disposable artifacts copy");
    for (Class<?> c: List.of(fd.class,lm.class,wi.class,uu.class,eb.class,st.class,ij.class,nu.class,qj.class))
      if (!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar)) throw new IOException("unexpected native class origin: "+c);
    PrintStream out=System.out; System.setOut(new PrintStream(OutputStream.nullOutputStream()));
    LightUpdateOracle.W w=LightUpdateOracle.world(); Map<String,qj> regions=new HashMap<>();
    try {
      for(int cx=-1;cx<=1;cx++) for(int cz=-1;cz<=1;cz++) {
        String name="r."+Math.floorDiv(cx,32)+"."+Math.floorDiv(cz,32)+".mcr";
        Path path=input.resolve(name);
        if(Files.isSymbolicLink(path)||!Files.isRegularFile(path,LinkOption.NOFOLLOW_LINKS)||!path.toRealPath().getParent().equals(input)) throw new IOException("unsafe region copy");
        qj region=regions.get(name); if(region==null) {region=new qj(path.toFile()); regions.put(name,region);}
        try(DataInputStream stream=region.a(Math.floorMod(cx,32),Math.floorMod(cz,32))) {
          if(stream==null) throw new IOException("missing chunk "+cx+","+cz);
          ij tag=ij.b(stream); if(!(tag instanceof nu)) throw new IOException("invalid root"); nu n=((nu)tag).k("Level");
          if(n.e("xPos")!=cx||n.e("zPos")!=cz) throw new IOException("wrong chunk coordinate");
          byte[] blocks=n.j("Blocks"), meta=n.j("Data"), sky=n.j("SkyLight"), light=n.j("BlockLight"), height=n.j("HeightMap");
          if(blocks.length!=32768||meta.length!=16384||sky.length!=16384||light.length!=16384||height.length!=256) throw new IOException("bad NBT array length");
          lm c=w.c(cx,cz); c.b=blocks; c.e=new wi(meta); c.f=new wi(sky); c.g=new wi(light); c.h=height;
        }
      }
    } finally { for(qj r: regions.values()) r.b(); }
    String name="saved-grass", before=LightUpdateOracle.stateDigest(w); StringBuilder rows=new StringBuilder();
    rows.append("case\t"+name+"\tsteps=1\n");
    rows.append("domain\t"+name+"\tchunks_x=-1,0,1\tchunks_z=-1,0,1\ty=0..127\tdefault_block=0\tdefault_meta=0\tdefault_block_light=0\tdefault_sky=0@y0;15@y1..127(native-lm.c-all-air)\tdigest="+before+"\n");
    LightMaterialEditV2Oracle.stateRows(w,name,"before",rows);
    for(int x=-7;x<=-3;x++)for(int y=65;y<=68;y++)for(int z=2;z<=6;z++) rows.append("sample\t").append(LightUpdateOracle.row(w,name,"before",x,y,z)).append('\n');
    if(w.a(-5,66,4)!=2 || w.a(-5,67,4)!=0) throw new IOException("saved state does not match logged grass/air target");
    rows.append("edit\t"+name+"\tafter-place\tplace\t-5\t67\t4\t1\t0\n");
    LightMaterialEditV2Oracle.put(w,new LightMaterialEditV2Oracle.Cell(-5,67,4,1,0));
    LightMaterialEditV2Oracle.stateRows(w,name,"after-place",rows);
    for(int x=-7;x<=-3;x++)for(int y=65;y<=68;y++)for(int z=2;z<=6;z++) rows.append("sample\t").append(LightUpdateOracle.row(w,name,"after-place",x,y,z)).append('\n');
    System.setOut(out); out.print(rows);
  }
}
