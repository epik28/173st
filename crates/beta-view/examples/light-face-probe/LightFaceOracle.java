import java.io.*;
import java.lang.reflect.*;
import java.nio.file.*;
import java.security.MessageDigest;
import java.util.*;

/** Headless direct native cube tessellation for a synthetic grass/stone edit. */
public class LightFaceOracle {
  static boolean edited;
  static int id(int x,int y,int z) {
    if (y==64 && z==4 && (x==4 || x==5)) return 2;
    if (edited && x==4 && y==66 && z==4) return 1;
    return 0;
  }
  static int light(int x,int y,int z) { return edited && x==4 && y==65 && z==4 ? 14 : 15; }
  static float brightness(int n) {
    float inv=1.0f-(float)n/15.0f;
    return ((1.0f-inv)/(inv*3.0f+1.0f))*0.95f+0.05f;
  }
  static int face(int[] a,int base,int x,int y,int z) {
    float[][] p=new float[4][3];
    for(int i=0;i<4;i++)for(int j=0;j<3;j++)p[i][j]=Float.intBitsToFloat(a[base+i*8+j]);
    for(int k=0;k<3;k++) {
      boolean equal=true; for(int i=1;i<4;i++) equal &= p[i][k]==p[0][k];
      if(equal) { float c=p[0][k]; if(k==0)return c==x?4:5; if(k==1)return c==y?0:1; return c==z?2:3; }
    }
    throw new IllegalStateException("non-axis-aligned quad");
  }
  public static void main(String[] args)throws Exception {
    Path jar=Path.of(args[0]).toRealPath();
    String hash=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
    if(!hash.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalStateException("wrong JAR");
    PrintStream output=System.out; System.setOut(new PrintStream(OutputStream.nullOutputStream())); uu grass=uu.m[2],stone=uu.m[1]; System.setOut(output);
    xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)-> {
      int x=a==null||a.length<3?0:(int)a[0], y=a==null||a.length<3?0:(int)a[1], z=a==null||a.length<3?0:(int)a[2];
      if(m.getReturnType()==float.class) return brightness(light(x,y,z));
      if(m.getReturnType()==int.class) return m.getName().equals("a")?id(x,y,z):0;
      if(m.getReturnType()==boolean.class) return id(x,y,z)!=0;
      if(m.getReturnType()==double.class) return 1d;
      return null;
    });
    cv renderer=new cv(world); cv.a=false; nw tess=nw.a;
    Field data=nw.class.getDeclaredField("g"),offset=nw.class.getDeclaredField("o");data.setAccessible(true);offset.setAccessible(true);int[] a=(int[])data.get(tess);tess.b();
    for(int phase=0;phase<2;phase++) { edited=phase==1;
      for(int[] pos:new int[][]{{4,64,4},{5,64,4},{4,66,4}}) {
        int x=pos[0],y=pos[1],z=pos[2],block=id(x,y,z);if(block==0)continue;
        offset.setInt(tess,0);renderer.b(block==1?stone:grass,x,y,z,1f,1f,1f);int end=offset.getInt(tess);
        if(end%48!=0) throw new IllegalStateException("unexpected quad layout");
        for(int i=0;i<end;i+=48) System.out.printf(Locale.ROOT,"%d\t%d\t%d\t%d\t%d\t%08x%n",phase,x,y,z,face(a,i,x,y,z),a[i+5]);
      }
    }
  }
}
