package org.lwjgl.opengl;
/** Record native matrix/state commands in invocation order; no GL context. */
public final class GL11 {
  public static void glPushMatrix() { System.out.println("push"); }
  public static void glPopMatrix() { System.out.println("pop"); }
  public static void glTranslatef(float x,float y,float z) { System.out.printf("translate\t%a\t%a\t%a%n",x,y,z); }
  public static void glScalef(float x,float y,float z) { System.out.printf("scale\t%a\t%a\t%a%n",x,y,z); }
  public static void glRotatef(float a,float x,float y,float z) { System.out.printf("rotate\t%a\t%a\t%a\t%a%n",a,x,y,z); }
  public static void glColor4f(float r,float g,float b,float a) { System.out.printf("color\t%a\t%a\t%a\t%a%n",r,g,b,a); }
  public static void glDisable(int v) { System.out.println("disable\t"+v); }
  public static void glEnable(int v) { System.out.println("enable\t"+v); }
}
