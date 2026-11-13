import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Direct native grass/stone flat renderer with arbitrary biome RGB; no GL calls. */
public class GrassTintFaceOracle {
  static boolean edited;
  static int id(int x,int y,int z) {
    if(y==64 && z==4 && (x==4 || x==5)) return 2;
    if(edited && x==4 && y==66 && z==4) return 1;
    return 0;
  }
  static int light(int x,int y,int z) {return edited && x==4 && y==65 && z==4 ? 14 : 15;}
  static float brightness(int n) {float inv=1f-(float)n/15f;return ((1f-inv)/(inv*3f+1f))*0.95f+0.05f;}
  public static void main(String[] args)throws Exception {
    Path jar=Path.of(args[0]).toRealPath();String hash=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
    if(!hash.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IOException("wrong client JAR");
    for(Class<?> c:List.of(xp.class,uu.class,cv.class,nw.class))if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("wrong class origin");
    PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu grass=uu.m[2],stone=uu.m[1];System.setOut(out);
    xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
      int x=a==null||a.length<3?0:(int)a[0],y=a==null||a.length<3?0:(int)a[1],z=a==null||a.length<3?0:(int)a[2];
      if(m.getReturnType()==float.class)return brightness(light(x,y,z));
      if(m.getReturnType()==int.class)return m.getName().equals("a")?id(x,y,z):0;
      if(m.getReturnType()==boolean.class)return id(x,y,z)!=0;
      if(m.getReturnType()==double.class)return 1d;return null;
    });
    cv renderer=new cv(world);cv.a=true;nw tess=nw.a;Field data=nw.class.getDeclaredField("g"),offset=nw.class.getDeclaredField("o");data.setAccessible(true);offset.setAccessible(true);int[] a=(int[])data.get(tess);tess.b();
    for(int tint:new int[]{0x3aa457,0xa1b2c3,0xffffff,0x050b13})for(int phase=0;phase<2;phase++){
      edited=phase==1;
      for(int[] pos:new int[][]{{4,64,4},{5,64,4},{4,66,4}}){int x=pos[0],y=pos[1],z=pos[2],block=id(x,y,z);if(block==0)continue;
        offset.setInt(tess,0);float r=block==2?((tint>>16)&255)/255f:1f,g=block==2?((tint>>8)&255)/255f:1f,b=block==2?(tint&255)/255f:1f;
        renderer.b(block==1?stone:grass,x,y,z,r,g,b);int end=offset.getInt(tess);if(end%48!=0)throw new IOException("unexpected quad layout");int[] layers=new int[6];
        for(int i=0;i<end;i+=48){int face=LightFaceOracle.face(a,i,x,y,z);int layer=layers[face]++;for(int v=0;v<4;v++)out.printf(Locale.ROOT,"%06x\t%d\t%d\t%d\t%d\t%d\t%d\t%d\t%08x%n",tint,phase,x,y,z,face,layer,v,a[i+v*8+5]);}
      }
    }
  }
}
