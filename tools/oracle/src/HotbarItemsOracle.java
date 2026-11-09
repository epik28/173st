import java.lang.reflect.Field;
import java.net.URL;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.util.HexFormat;
import javax.imageio.ImageIO;

/** Native item registry and original-font measurements, with no game window. */
public final class HotbarItemsOracle {
    static String sha(byte[] b) throws Exception { return HexFormat.of().formatHex(MessageDigest.getInstance("SHA-256").digest(b)); }
    static String source(Class<?> c) throws Exception { URL u=c.getProtectionDomain().getCodeSource().getLocation(); return Path.of(u.toURI()).toRealPath().toString(); }
    public static void main(String[] args) throws Exception {
        if(args.length!=1) throw new IllegalArgumentException("usage: HotbarItemsOracle <client.jar>");
        Path jar=Path.of(args[0]).toRealPath(); byte[] jb=java.nio.file.Files.readAllBytes(jar);
        if(!sha(jb).equals("af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c")) throw new IllegalStateException("wrong pinned JAR");
        java.io.PrintStream out=System.out;
        try { System.setOut(new java.io.PrintStream(java.io.OutputStream.nullOutputStream())); Class.forName("uu"); }
        finally { System.setOut(out); }
        Field icon=gm.class.getDeclaredField("bh"); icon.setAccessible(true);
        byte[] png; try(var z=new java.util.zip.ZipFile(jar.toFile())) { png=z.getInputStream(z.getEntry("font/default.png")).readAllBytes(); }
        var image=ImageIO.read(new java.io.ByteArrayInputStream(png)); Field mapField=fp.class.getDeclaredField("a"); mapField.setAccessible(true); String map=(String)mapField.get(null);
        int[] widths=new int[10]; for(int d=0;d<10;d++){ int i=map.indexOf('0'+d), right=-1; for(int col=7;col>=0&&right<0;col--) for(int y=0;y<8;y++) if((image.getRGB((i%16)*8+col,(i/16)*8+y)&255)>0){ right=col; break; } widths[d]=right<0?2:right+2; }
        out.println("jar_sha256\t"+sha(jb)); out.println("code_source\t"+source(iz.class));
        out.print("digit_widths"); for(int w:widths) out.print("\t"+w); out.println();
        for(int id:new int[]{1,3,332,267}) { iz stack=new iz(id,1,0); gm item=stack.a(); if(item==null) throw new IllegalStateException("missing native id "+id); out.println("icon\t"+id+"\t"+item.getClass().getName()+"\t"+icon.getInt(item)); }
    }
}
