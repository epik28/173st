import java.io.*;import java.lang.reflect.*;import java.nio.file.*;import java.security.*;import java.util.*;
/** Direct native tall grass render; deterministic synthetic climate and palette. */
public final class TallGrassOracle {
 static int meta, sx, sz;
 static class Climate extends xv { Climate(){super();a=new double[]{0.5};b=new double[]{0.5};} public kd[] a(int x,int z,int w,int h){sx=x;sz=z;return new kd[1];} }
 static String hex(int x){return String.format(Locale.ROOT,"%08x",x);}
 public static void main(String[] args)throws Exception{
  if(args.length!=1)throw new IllegalArgumentException("CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();
  String sha=HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(Files.readAllBytes(jar)));
  if(!sha.equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"))throw new IllegalArgumentException("wrong jar");
  for(Class<?> c:List.of(cv.class,uu.class,nw.class,xp.class,ru.class,ia.class))if(!Path.of(c.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IllegalArgumentException("wrong origin");
  PrintStream out=System.out;System.setOut(new PrintStream(OutputStream.nullOutputStream()));uu grass=uu.m[31];int[] palette=new int[65536];Arrays.fill(palette,0x76923d);ia.a(palette);System.setOut(out);
  Climate climate=new Climate();xp world=(xp)Proxy.newProxyInstance(xp.class.getClassLoader(),new Class[]{xp.class},(p,m,a)->{
   Class<?> type=m.getReturnType();if(type==int.class)return m.getName().equals("e")?meta:0;if(type==float.class)return 1f;if(type==boolean.class)return false;if(type==xv.class)return climate;return null;});
  cv renderer=new cv(world);nw tess=nw.a;Field dataF=nw.class.getDeclaredField("g"),offsetF=nw.class.getDeclaredField("o");dataF.setAccessible(true);offsetF.setAccessible(true);int[] data=(int[])dataF.get(tess);tess.b();
  int[][] coords={{0,0,0},{5,64,-7},{-17,72,31},{1234,127,-4321},{-2147483648,1,2147483647}};
  out.println("# native beta tall grass, fixed climate/palette, brightness=1");
  for(int ci=0;ci<coords.length;ci++)for(meta=0;meta<16;meta++){
   int x=coords[ci][0],y=coords[ci][1],z=coords[ci][2];sx=sz=Integer.MIN_VALUE;offsetF.setInt(tess,0);boolean rendered=renderer.b(grass,x,y,z);int n=offsetF.getInt(tess);
   out.println("case\t"+ci+"\t"+meta+"\t"+x+"\t"+y+"\t"+z+"\t"+sx+"\t"+sz+"\t"+(rendered?1:0)+"\t"+(n/8));
   for(int i=0;i<n;i+=8){StringJoiner row=new StringJoiner("\t");row.add("vertex").add(Integer.toString(ci)).add(Integer.toString(meta)).add(Integer.toString(i/8));for(int j=0;j<8;j++)row.add(hex(data[i+j]));out.println(row);}
  }
 }
}
