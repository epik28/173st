#!/usr/bin/env python3
"""Static, read-only C01 audit of the Beta 1.7.3 world selection GUI."""
from pathlib import Path
import hashlib, json, subprocess, zipfile

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
EXPECTED_JAR = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASSES = {
    "fu": "baa1bc76a2451de306229c87a2964b48abf09c0b16353c25f1bb4cdcff79cb2c",
    "rq": "7f3fdc2729f5895cf414601075fa847d61bb7a719f73d60a39c69bf97a16c90f",
    "mg": "0b02e04a20ed97e80a01c8733d13c58937d193a328adaaa568a5a5e53958b4fe",
    "vb": "ff8c4833109f9bd4439ebc4ddb718ad303fec9074a2f66a744f06de754d169a9",
    "tq": "339175cb6bdd4f97024f30c9c501b4efe9221f60672a9fb3b017890b5b2272c8",
}

def javap(name):
    return subprocess.check_output(["javap", "-classpath", str(JAR), "-c", "-private", name], text=True)

def audit():
    jar_hash = hashlib.sha256(JAR.read_bytes()).hexdigest()
    assert jar_hash == EXPECTED_JAR, (jar_hash, EXPECTED_JAR)
    with zipfile.ZipFile(JAR) as z:
        hashes = {n: hashlib.sha256(z.read(n)).hexdigest() for n in (f"{c}.class" for c in CLASSES)}
    assert {c: hashes[f"{c}.class"] for c in CLASSES} == CLASSES
    fu, rq, mg, vb, tq = map(javap, CLASSES)
    checks = {
        "main_singleplayer_opens_world_select": "class rq" in fu and "menu.singleplayer" in fu,
        "worlds_loaded_and_sorted": "nl.b" in rq and "Collections.sort" in rq,
        "selection_starts_unset": "iconst_m1" in rq,
        "list_rows_36px": "bipush        36" in mg and "bipush        36" in mg,
        "double_click_loads": "rq.e" in mg,
        "select_button": "selectWorld.select" in rq and "sipush        154" in rq,
        "create_button": "selectWorld.create" in rq and "iconst_3" in rq,
        "cancel_button": "gui.cancel" in rq and "iconst_0" in rq,
        "cancel_returns_parent": "Field a:Lda" in rq,
        "loading_guard": "Field l:Z" in rq and "putfield" in rq,
        "save_summary_sort": "lcmp" in vb and "compareTo" in vb,
        "old_format_five_slots": "iconst_5" in tq and "World" in tq,
    }
    assert all(checks.values()), [k for k, v in checks.items() if not v]
    # Mutation boundary: this audit only hashes/reads bytecode and never opens a save.
    return {"jar_sha256": jar_hash, "class_sha256": CLASSES, "checks": checks,
            "scope": "C01 world selection/list/select/cancel", "mutations": []}

if __name__ == "__main__":
    print(json.dumps(audit(), sort_keys=True, indent=2))
