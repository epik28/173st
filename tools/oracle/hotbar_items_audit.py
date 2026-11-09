#!/usr/bin/env python3
"""Strict native audit of Beta 1.7.3 hotbar item icons and count geometry."""
import hashlib, json, shutil, subprocess, tempfile, zipfile
from pathlib import Path

ROOT=Path(__file__).resolve().parents[2]
JAR_SHA="af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
FIXTURE=ROOT/"tests/fixtures/hotbar_items/audit.json"
ORACLE=ROOT/"tools/oracle/src/HotbarItemsOracle.java"
CLASSES={"uq":"b4b61c370315e1ed95b42a24a01112afd7c2d44e387e48adc5e801edd4578f12","bb":"696e7e8a848ebdbc92979be779b42ab87c39b91fd9ec6b6f7a239a8dc4a792b6","sj":"c1bec862504ca9b390810ac6eb4e3401032614bf24560757ffb11533ddb66ad3","fp":"3749b3ed11fd0ab6fd46c68645a897d6132350aa87346c745c220ce42f696e77","iz":"3e8a07bf8c43729c8367bab454af876e698505620f230e012432c04e3828d6d6","gm":"f9d971a83efa0c6399f1fc4bc0bc06c6a6249908afc6b46f5889a49365c190c7","ck":"fe3d757cd56ac42ef8e0c0aaecf5fc7878406fe66db88ecf8061456f50cdd2eb","dm":"c62e04938cf3b6d54ebf1de986fabc6f8fe366e74e3cb70bc1c4a053370c5f89","qd":"5721cdbdbf6f52606e7710df443cb19a573318cd5fd1913aecb40933e9119f56"}
RES={"gui/items.png":"545699e0c5f655c1e1876bb9f2fa5d93c4fd514dae3802d1cea43ae0003343f3","terrain.png":"9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf","font/default.png":"6ded9b9e06a61beaa61bf8542c46b5d690d45fdaf42e6539afcbc02e4c7a6408","font.txt":"cf0eed8e0915ce2f794a521b90eabec0cad62df39b18bbd2a986f475e1c7a8c2"}
EVIDENCE={
"hud_method":"descriptor: (FZII)V","slot_loop":"1015: invokespecial #202                // Method a:(IIIF)V","slot_x":"981: iload         6\n     983: iconst_2\n     984: idiv\n     985: bipush        90\n     987: isub\n     988: iload         15\n     990: bipush        20\n     992: imul\n     993: iadd\n     994: iconst_2\n     995: iadd","slot_y":"998: iload         7\n    1000: bipush        16\n    1002: isub\n    1003: iconst_3\n    1004: isub","render_slot":"descriptor: (IIIF)V","icon_call":"118: invokevirtual #139                // Method bb.a:(Lsj;Lji;Liz;II)V","count_call":"152: invokevirtual #140                // Method bb.b:(Lsj;Lji;Liz;II)V","terrain_branch":"233: aload_2\n     234: aload_2\n     235: ldc           #24                 // String /terrain.png","item_branch":"246: aload_2\n     247: aload_2\n     248: ldc           #23                 // String /gui/items.png","atlas_threshold":"226: iload_3\n     227: sipush        256\n     230: if_icmpge     246","uv":"334: bipush        16\n     336: irem\n     337: bipush        16\n     339: imul","count_guard":"9: iconst_1\n      10: if_icmple     90","count_text":"52: iload         4\n      54: bipush        19\n      56: iadd\n      57: iconst_2\n      58: isub\n      59: aload_1\n      60: aload         6\n      62: invokevirtual #106                // Method sj.a:(Ljava/lang/String;)I","count_xy":"66: iload         5\n      68: bipush        6\n      70: iadd\n      71: iconst_3\n      72: iadd","font_scan":"176: iaload\n     177: sipush        255\n     180: iand\n     181: istore        16","font_width":"225: iload         8\n     227: iload         11\n     229: iconst_2\n     230: iadd\n     231: iastore"
}
ROWS=[["stone",1,1,0,0,"ck",1,"/terrain.png",16,0,339,461,"",0,-1,-1],["dirt",3,1,4,0,"ck",2,"/terrain.png",32,0,419,461,"",0,-1,-1],["arrow_stack",332,2,8,0,"dm",14,"/gui/items.png",224,0,499,461,"2",8,508,470],["arrow_full",332,16,4,0,"dm",14,"/gui/items.png",224,0,419,461,"16",16,420,470],["iron_pick",267,1,0,0,"qd",66,"/gui/items.png",32,64,339,461,"",0,-1,-1]]
WIDTHS=[8,8,8,8,9,9,8,9,8,8]
def sha(b): return hashlib.sha256(b).hexdigest()
def load_fixture(path=FIXTURE):
 try: v=json.loads(Path(path).read_text())
 except (OSError,ValueError) as e: raise ValueError(f"invalid hotbar items fixture: {e}") from e
 if v.get("schema_version")!=1 or v.get("jar_sha256")!=JAR_SHA or v.get("classes")!=CLASSES or v.get("resources")!=RES or v.get("bytecode_evidence")!=EVIDENCE or v.get("digit_widths")!=WIDTHS or v.get("rows")!=ROWS: raise ValueError("fixture differs from pinned native hotbar evidence")
 return v
def verify(jar):
 jar=Path(jar); v=load_fixture(); data=jar.read_bytes()
 if sha(data)!=JAR_SHA: raise ValueError("client JAR hash differs from pinned Beta 1.7.3")
 with zipfile.ZipFile(jar) as z:
  for n,h in RES.items():
   if sha(z.read(n))!=h: raise ValueError(f"resource hash differs: {n}")
  for n,h in CLASSES.items():
   if sha(z.read(n+".class"))!=h: raise ValueError(f"class hash differs: {n}")
 javap=shutil.which("javap"); javac=shutil.which("javac"); java=shutil.which("java")
 if not all((javap,javac,java)): raise ValueError("JDK tools javap, javac and java required")
 out={}
 for c in ["uq","bb","sj"]:
  p=subprocess.run([javap,"-classpath",str(jar),"-p","-s","-c",c],capture_output=True,text=True,timeout=30)
  if p.returncode: raise ValueError("javap failed: "+c)
  out[c]=p.stdout
 for k,s in EVIDENCE.items():
  c="uq" if k in ("hud_method","slot_loop","slot_x","slot_y","render_slot","icon_call","count_call") else "sj" if k.startswith("font_") else "bb"
  if " ".join(s.split()) not in " ".join(out[c].split()): raise ValueError("native bytecode evidence missing/changed: "+k)
 with tempfile.TemporaryDirectory(prefix="hotbar-items-") as t:
  t=Path(t); (t/"classes").mkdir(); src=subprocess.run([javac,"-cp",str(jar),"-d",str(t/"classes"),str(ORACLE)],capture_output=True,text=True,timeout=30)
  if src.returncode: raise ValueError("oracle compile failed: "+src.stderr)
  outputs=[]
  for _ in range(2):
   p=subprocess.run([java,"-cp",f"{t/'classes'}:{jar}","HotbarItemsOracle",str(jar)],capture_output=True,text=True,timeout=30)
   if p.returncode: raise ValueError("native oracle failed: "+p.stderr)
   outputs.append(p.stdout)
  if outputs[0]!=outputs[1]: raise ValueError("native oracle repeat output differs")
  lines=outputs[0].splitlines()
  if lines[0]!=f"jar_sha256\t{JAR_SHA}" or Path(lines[1].split("\t",1)[1]).resolve()!=jar.resolve(): raise ValueError("oracle source/JAR provenance differs")
  if list(map(int,lines[2].split("\t")[1:]))!=WIDTHS: raise ValueError("native font digit widths differ")
  icons=[[int(p[1]),p[2],int(p[3])] for p in (line.split("\t") for line in lines[3:])]
  if icons!=[[1,"ck",1],[3,"ck",2],[332,"dm",14],[267,"qd",66]]: raise ValueError("native item icon rows differ")
 return v
if __name__=="__main__":
 import argparse
 p=argparse.ArgumentParser(); p.add_argument("jar",type=Path); a=p.parse_args(); verify(a.jar); print("hotbar-items native audit PASS")
