#!/usr/bin/env python3
"""Static C01 audit of Beta 1.7.3 first-person block interaction bytecode."""
from pathlib import Path
import hashlib
import json
import subprocess
import zipfile

ROOT = Path(__file__).resolve().parents[2]
JAR = ROOT.parent / ".local/minecraft/client-b1.7.3.jar"
EXPECTED_JAR = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASSES = {
    "net/minecraft/client/Minecraft.class": "db99df969e5f30d01ad4424a4d63d3b1f4a11832055dd291e65d476cfa4c38b3",
    "ob.class": "c367f0a0b4467234a3d30690106b2adde8ad74f7c80db285fefe1f75101c3147",
    "pj.class": "c2735d4e58bdbcf882d930f4b8925df7c218389e7a4c378319a444d303d92590",
    "uu.class": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
    "dn.class": "a4992db8f7765066458ef19fd471933a0ac5af61257bd1dbe08f67a5be69b57d",
}

def javap(name):
    return subprocess.check_output(["javap", "-classpath", str(JAR), "-c", "-private", name], text=True)

def audit():
    jar_hash = hashlib.sha256(JAR.read_bytes()).hexdigest()
    assert jar_hash == EXPECTED_JAR, (jar_hash, EXPECTED_JAR)
    with zipfile.ZipFile(JAR) as z:
        actual = {name: hashlib.sha256(z.read(name)).hexdigest() for name in CLASSES}
    assert actual == CLASSES
    mc, ob, pj, uu, dn = (javap("net.minecraft.client.Minecraft"), javap("ob"),
                          javap("pj"), javap("uu"), javap("dn"))
    checks = {
        "mouse_button_dispatch": "Mouse.getEventButton" in mc and "Method a:(I)V" in mc,
        "held_update_dispatch": "Method ob.a:(F)V" in mc,
        "rmb_placement_descriptor": "a(gs, fd, iz, int, int, int, int)" in ob,
        "placement_delegates_item": "Method iz.a:(Lgs;Lfd;IIII)Z" in ob,
        "break_writes_effect_2001": "sipush        2001" in ob,
        "break_calls_block_callback": "Method uu.c:(Lfd;IIII)V" in ob,
        "reach_is_five": "float 5.0f" in ob,
        "hardness_formula": "Method gs.b:(Luu;)Z" in uu and "float 100.0f" in uu and "float 30.0f" in uu,
        "drop_suppressed_creative_flag": "Field fd.B:Z" in uu,
        "creative_controller_is_pj": "class pj extends ob" in pj and "putfield      #18                 // Field b:Z" in pj,
        "creative_inventory_fill": "Field gr.a" in pj,
        "held_release_method": "private void a(int, boolean)" in mc,
        "held_lmb_repeat": "Mouse.isButtonDown:(I)Z" in mc and "Method a:(I)V" in mc,
        "repeat_tick_divisor": "float 4.0f" in mc and "Field V:I" in mc and "Field ag:I" in mc,
        "release_resets_cooldown": "putfield      #287                // Field W:I" in mc,
        "block_hit_gate": "Field vf.a:Ljg" in mc and "Field jg.a:Ljg" in mc,
        "held_target_controller_call": "Method ob.c:(IIII)V" in mc,
        "held_target_overlay_call": "Method dn.a:(IIII)V" in mc,
        "target_loss_controller_reset": "Method ob.a:()V" in mc,
        "gui_suppresses_gameplay_input": "Field r:Lda" in mc and "ifnonnull" in mc,
        "overlay_four_int_descriptor": "a(int, int, int, int)" in dn,
    }
    assert all(checks.values()), [k for k, v in checks.items() if not v]
    return {
        "jar_sha256": jar_hash,
        "class_sha256": actual,
        "checks": checks,
        "descriptors": {
            "Minecraft.mouse_action": "(I)V",
            "Minecraft.controller_tick": "(F)V",
            "ob.start_or_break": "(IIII)V",
            "ob.remove_block": "(IIII)Z",
            "ob.place_item": "(Lgs;Lfd;Liz;IIII)Z",
            "uu.break_fraction": "(Lgs;)F",
            "Minecraft.held_update": "(IZ)V",
            "ob.reset": "()V",
            "ob.held_target": "(IIII)V",
            "dn.break_overlay": "(IIII)V",
        },
        "scope": "C01 native first-person mining/placement audit",
        "mutations": [],
        "caveats": [
            "pj is the creative controller; no separate ob subclass is present in the pinned client jar.",
            "ob.a(float) is a no-op in this jar, so progressive state must be captured from the caller/controller path before implementation.",
        ],
    }

if __name__ == "__main__":
    print(json.dumps(audit(), sort_keys=True, indent=2))
