# Mining and placement C01 audit

This directory records the native audit boundary for first-person block interaction in
the pinned Beta 1.7.3 client. `tools/oracle/mining_audit.py` hashes the client JAR and
the interaction classes, then checks bytecode facts with `javap`.

The captured native entry points are `Minecraft.a(int)` for mouse dispatch,
`Minecraft.a(int,boolean)` for held/released interaction, `ob.a()` for reset,
`ob.c(int,int,int,int)` for held block targets, `dn.a(int,int,int,int)` for the
break overlay, `ob.a(gs,fd,iz,int,int,int,int)` for placement,
`ob.b(int,int,int,int)` for removal, and `uu.a(gs)` for break fraction. The caller
checks a block hit, suppresses gameplay input while a GUI is open, repeats held
buttons every `py.a / 4` ticks, and resets on release or target loss. The audit is
read-only and reports an empty mutation list. `pj` is the creative controller; the
pinned jar has no additional direct subclass of `ob`.
