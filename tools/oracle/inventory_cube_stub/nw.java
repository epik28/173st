/** Records tessellator submissions from the original cv renderer. */
public final class nw {
  public static final nw a = new nw();
  public void b() { System.out.println("begin"); }
  public void a() { System.out.println("draw"); }
  public void b(float x,float y,float z) { System.out.printf("normal\t%a\t%a\t%a%n",x,y,z); }
  public void a(double x,double y,double z,double u,double v) { System.out.printf("vertex\t%a\t%a\t%a\t%a\t%a%n",x,y,z,u,v); }
}
