package org.lwjgl.opengl;
/** No-op GL state calls for headless overlay capture. */
public final class GL11 {
    public static void glDisable(int x) { }
    public static void glEnable(int x) { }
    public static void glColor4f(float r, float g, float b, float a) { }
}
