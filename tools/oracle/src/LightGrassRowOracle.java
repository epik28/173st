import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.PrintStream;
import java.nio.charset.StandardCharsets;
import java.nio.file.Path;
import java.util.*;

/** Full-state sequential synthetic probes through the pinned native Beta light update path. */
public final class LightGrassRowOracle {
  static final String REQUEST = "material-edits-v2\n";
  record Cell(int x,int y,int z,int id,int meta) {}
  record Point(int x,int y,int z) {}
  record Step(String phase,String operation,Cell cell) {}
  record Case(String name,List<Cell> seed,List<Step> steps,List<Point> samples) {}
  record State(int id,int meta,int block,int sky) {}
  record Height(int value) {}
  static final State AIR0=new State(0,0,0,0), AIR15=new State(0,0,0,15);

  static void put(LightUpdateOracle.W w,Cell c)throws Exception {

    LightUpdateOracle.obj(w,fd.class,"r",new java.util.Random(0L));
    LightUpdateOracle.obj(w,fd.class,"b",new java.util.ArrayList<>());
    LightUpdateOracle.obj(w,fd.class,"D",new java.util.ArrayList<>());
    LightUpdateOracle.integer(w,fd.class,"q",0);
    boolean changed=w.a(c.x,c.y,c.z,c.id,c.meta);
    if(!changed && w.a(c.x,c.y,c.z)!=c.id)throw new IOException("native block setter rejected cell "+c);
    LightUpdateOracle.settle(w,c.x,c.y,c.z);
  }
  static Case[] cases(){
    List<Cell> floor=new ArrayList<>();
    for(int x=4;x<=12;x++)for(int z=4;z<=12;z++) {
      floor.add(new Cell(x,62,z,3,0));
      floor.add(new Cell(x,63,z,2,0));
    }
    List<Step> steps=new ArrayList<>();
    for(int x=7;x<=10;x++) steps.add(new Step("placed-"+x,"place",new Cell(x,64,8,1,0)));
    for(int x=7;x<=10;x++) steps.add(new Step("removed-"+x,"remove",new Cell(x,64,8,0,0)));
    return new Case[]{new Case("grass-row-place-remove",floor,steps,
      List.of(new Point(7,63,8),new Point(7,64,8),new Point(8,64,8),new Point(9,64,8),new Point(10,64,8),new Point(11,64,8)))};
  }
  static String stateRows(LightUpdateOracle.W w,String name,String phase,StringBuilder out)throws Exception {
    String digest=LightUpdateOracle.stateDigest(w);out.append("snapshot\t").append(name).append('\t').append(phase).append("\tsha256=").append(digest).append('\n');
    for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++){lm c=w.c(cx,cz);for(int y=0;y<128;y++)for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++){int id=c.a(lx,y,lz),meta=c.e.a(lx,y,lz),block=c.g.a(lx,y,lz),sky=c.f.a(lx,y,lz),def=y==0?0:15;if(id!=0||meta!=0||block!=0||sky!=def)out.append("state\t").append(name).append('\t').append(phase).append('\t').append(cx*16+lx).append('\t').append(y).append('\t').append(cz*16+lz).append('\t').append(id).append('\t').append(meta).append('\t').append(block).append('\t').append(sky).append('\n');}for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++){int h=c.h[lx*16+lz];if(h!=0)out.append("height\t").append(name).append('\t').append(phase).append('\t').append(cx*16+lx).append('\t').append(cz*16+lz).append('\t').append(h).append('\n');}}
    return digest;
  }
  static Map<String,State> states(LightUpdateOracle.W w){Map<String,State> m=new TreeMap<>();for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++){lm c=w.c(cx,cz);for(int y=0;y<128;y++)for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++){int id=c.a(lx,y,lz),md=c.e.a(lx,y,lz),bl=c.g.a(lx,y,lz),sk=c.f.a(lx,y,lz);if(id!=0||md!=0||bl!=0||sk!=(y==0?0:15))m.put((cx*16+lx)+","+y+","+(cz*16+lz),new State(id,md,bl,sk));}}return m;}
  static Map<String,Integer> heights(LightUpdateOracle.W w){Map<String,Integer> m=new TreeMap<>();for(int cx=-1;cx<=1;cx++)for(int cz=-1;cz<=1;cz++){lm c=w.c(cx,cz);for(int lx=0;lx<16;lx++)for(int lz=0;lz<16;lz++)if(c.h[lx*16+lz]!=0)m.put((cx*16+lx)+","+(cz*16+lz),c.h[lx*16+lz] & 255);}return m;}
  static void diff(String name,String from,String to,Map<String,State>a,Map<String,Integer>ha,Map<String,State>b,Map<String,Integer>hb,StringBuilder out){Set<String> keys=new TreeSet<>();keys.addAll(a.keySet());keys.addAll(b.keySet());for(String k:keys){String[] p=k.split(",");int y=Integer.parseInt(p[1]);State def=y==0?AIR0:AIR15,x=a.getOrDefault(k,def),z=b.getOrDefault(k,def);if(!x.equals(z))out.append("delta\t").append(name).append('\t').append(from).append('\t').append(to).append('\t').append(k.replace(',', '\t')).append('\t').append(x.id).append('\t').append(x.meta).append('\t').append(x.block).append('\t').append(x.sky).append('\t').append(z.id).append('\t').append(z.meta).append('\t').append(z.block).append('\t').append(z.sky).append('\n');}keys.clear();keys.addAll(ha.keySet());keys.addAll(hb.keySet());for(String k:keys){int x=ha.getOrDefault(k,0),z=hb.getOrDefault(k,0);if(x!=z)out.append("hdelta\t").append(name).append('\t').append(from).append('\t').append(to).append('\t').append(k.replace(',', '\t')).append('\t').append(x).append('\t').append(z).append('\n');}}
  static void samples(LightUpdateOracle.W w,Case c,String phase,StringBuilder out){for(Point p:c.samples)out.append("sample\t").append(LightUpdateOracle.row(w,c.name,phase,p.x,p.y,p.z)).append('\n');}
  static String domain(Case c,String digest){StringBuilder s=new StringBuilder("domain\t").append(c.name).append("\tchunks_x=-1,0,1\tchunks_z=-1,0,1\ty=0..127\tdefault_block=0\tdefault_meta=0\tdefault_block_light=0\tdefault_sky=0@y0;15@y1..127(native-lm.c-all-air)\tdefault_height=0(native-lm.c-all-air)\tmaterial=uu.q[id],uu.s[id]\tinit=xa.a(0);lm.c();fd.j-quiescent\tdigest=").append(digest).append('\n');for(int i=0;i<c.seed.size();i++){Cell a=c.seed.get(i);s.append("seed\t").append(c.name).append('\t').append(i).append('\t').append(a.x).append('\t').append(a.y).append('\t').append(a.z).append('\t').append(a.id).append('\t').append(a.meta).append('\n');}return s.toString();}
  public static void main(String[] args)throws Exception{if(args.length!=1)throw new IOException("usage: LightGrassRowOracle CLIENT_JAR");Path jar=Path.of(args[0]).toRealPath();if(!LightUpdateOracle.sha(jar).equals(LightUpdateOracle.JAR_SHA))throw new IOException("wrong client JAR SHA-256");for(Class<?> t:new Class<?>[]{fd.class,lm.class,wi.class,uu.class,eb.class,st.class})if(!Path.of(t.getProtectionDomain().getCodeSource().getLocation().toURI()).toRealPath().equals(jar))throw new IOException("native class outside pinned JAR: "+t.getName());byte[] in=System.in.readNBytes(REQUEST.length()+1);if(!Arrays.equals(in,REQUEST.getBytes(StandardCharsets.US_ASCII))||System.in.read()!=-1)throw new IOException("request must be exactly material-edits-v2\n");StringBuilder out=new StringBuilder();PrintStream real=System.out;ByteArrayOutputStream boot=new ByteArrayOutputStream();System.setOut(new PrintStream(boot,true,StandardCharsets.UTF_8));try{for(Case c:cases()){LightUpdateOracle.W w=LightUpdateOracle.world();for(Cell seed:c.seed)put(w,seed);String before=LightUpdateOracle.stateDigest(w);out.append("case\t").append(c.name).append("\tsteps=").append(c.steps.size()).append("\n").append(domain(c,before));Map<String,State> prev=states(w);Map<String,Integer> ph=heights(w);stateRows(w,c.name,"before",out);samples(w,c,"before",out);String priorPhase="before";for(Step step:c.steps){Cell e=step.cell;out.append("edit\t").append(c.name).append('\t').append(step.phase).append('\t').append(step.operation).append('\t').append(e.x).append('\t').append(e.y).append('\t').append(e.z).append('\t').append(e.id).append('\t').append(e.meta).append('\n');put(w,e);Map<String,State> next=states(w);Map<String,Integer> nh=heights(w);diff(c.name,priorPhase,step.phase,prev,ph,next,nh,out);priorPhase=step.phase;stateRows(w,c.name,step.phase,out);samples(w,c,step.phase,out);prev=next;ph=nh;}}}finally{System.setOut(real);}if(!LightUpdateOracle.shaBytes(boot.toByteArray()).equals("0b280c253ec80f2690447bfc42cadac3ed78f1d388bdbe2d10b340c76daacc4e"))throw new IOException("unexpected native bootstrap output");System.out.write(out.toString().getBytes(StandardCharsets.US_ASCII));}
}
