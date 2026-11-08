#!/usr/bin/env python3
"""Generate deterministic movement inputs and capture the pinned client's native results."""
import hashlib
import json
import math
from pathlib import Path
import random
import struct
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/movement"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
CLASS_SHA = {
    "sn": "b53107a5bdd5ff92637582aaed4fd49366443a11cba96cef0a6ad719e197b7be",
    "ls": "b70fbb1499cd1b37ef27dd5dec7637c7338439ef03fe13012dca93fb2326d078",
    "eq": "81a4cf4a682b2c830e78160d944aa71342e2f6b6d415863c9f18600b08cd191f",
    "bt": "32018dcec6f1af1efa93baac09178456d9d2537367721875dae6547588a308f8",
    "in": "8693f3179fb0ff1df382e1d3388d23ab688ad3e26415454655da88f10b054f63",
    "uu": "caeaf354357d66fc168c7f03bf62bb6c6a1fe459e9be6b537d4d5d1efabd0845",
}
METHOD_MAPPING = {
    "move": {"class": "sn", "name": "b", "descriptor": "(DDD)V"},
    "travel": {"class": "ls", "name": "a_", "descriptor": "(FF)V"},
    "jump": {"class": "ls", "name": "R", "descriptor": "()V"},
}
FIELD_MAPPING = {
    "position_x": "aM", "position_y": "aN", "position_z": "aO",
    "velocity_x": "aP", "velocity_y": "aQ", "velocity_z": "aR",
    "bbox_min_x": "aW.a", "bbox_min_y": "aW.b", "bbox_min_z": "aW.c",
    "bbox_max_x": "aW.d", "bbox_max_y": "aW.e", "bbox_max_z": "aW.f",
    "y_offset": "bf", "y_size": "bo", "step_height": "bp", "fall_distance": "bk",
    "yaw_degrees": "aS", "on_ground": "aX", "collided_horizontally": "aY",
    "collided_vertically": "aZ", "collided": "ba", "no_clip": "bq", "in_web": "bc",
    "sneaking": "t()", "allow_slide": "bd",
}
SURFACES = {"1": "3f19999a", "79": "3f7ae148"}


def sha(data):
    return hashlib.sha256(data).hexdigest()


def raw64(value):
    return struct.pack(">d", float(value)).hex()


def f32(value):
    return struct.unpack(">f", struct.pack(">f", float(value)))[0]


def raw32(value):
    return struct.pack(">f", f32(value)).hex()


def parse64(raw):
    return struct.unpack(">d", bytes.fromhex(raw))[0]


def state(x=0.0, y=1.0, z=0.0, velocity=(0.0, 0.0, 0.0),
          y_offset=0.0, y_size=0.0, step_height=0.5, fall_distance=0.0,
          yaw=0.0, flags=128, bbox=None):
    x, y, z = float(x), float(y), float(z)
    if bbox is None:
        half = f32(f32(0.6) / 2.0)
        height = f32(1.8)
        bottom = y - float(f32(y_offset)) + float(f32(y_size))
        bbox = (x-half, bottom, z-half, x+half, bottom+height, z+half)
    values = (x, y, z, *velocity, *bbox)
    floats = (y_offset, y_size, step_height, fall_distance, yaw)
    return ",".join([*(raw64(v) for v in values), *(raw32(v) for v in floats), str(flags)])


def box(*coords):
    return ",".join(raw64(v) for v in coords)


def scenario(case_id, mode, initial, boxes, surface, frames):
    box_field = "-" if not boxes else ":".join(box(*b) for b in boxes)
    return "\t".join((case_id, mode, initial, box_field, str(surface), ";".join(frames)))


def move_frame(x, y, z):
    return ",".join(raw64(v) for v in (x, y, z))


def travel_frame(strafe, forward, yaw, jump=False, sneak=False):
    return ",".join((raw32(strafe), raw32(forward), raw32(yaw), "1" if jump else "0", "1" if sneak else "0"))


def corpus():
    rows = []
    floor = (-16.0, -8.0, -16.0, 16.0, 1.0, 16.0)
    special = [
        ("flat-ground", state(flags=1|128), [floor], 1, [(0,0,0),(0,0,0)]),
        ("stone-wall", state(x=0.65, flags=1|128), [floor,(1,1,-1,2,3,1)], 1, [(1,0,0)]),
        ("step-low", state(x=0.65, flags=1|128), [floor,(1,1,-1,1.5,1.4,1)], 1, [(1,0,0)]),
        ("step-high", state(x=0.65, step_height=0.75, flags=1|128), [floor,(1,1,-1,1.5,1.6,1)], 1, [(1,0,0)]),
        ("step-disabled", state(x=0.65, step_height=0.0, flags=1|128), [floor,(1,1,-1,1.5,1.4,1)], 1, [(1,-0.01,0)]),
        ("step-tie", state(x=0.65, flags=1|128), [floor,(1,1,-1,1.5,1.4,1),(-2,2.8,-2,3,4,2)], 1, [(1,-0.01,0)]),
        ("step-tie-disabled", state(x=0.65, step_height=0.0, flags=1|128), [floor,(1,1,-1,1.5,1.4,1),(-2,2.8,-2,3,4,2)], 1, [(1,-0.01,0)]),
        ("ceiling", state(y=2.2), [( -3,-8,-3,3,1,3),(-2,3,-2,2,4,2)], 1, [(0,2,0)]),
        ("sneak-ledge", state(x=0.35, flags=1|64|128), [(-4,-8,-4,0.4,1,4)], 1, [(1,0,0),(1,0,0)]),
        ("sneak-corner", state(x=0.35,z=0.35,flags=1|64|128), [(-4,-8,-4,0.4,1,0.4)], 1, [(1,0,1)]),
        ("sneak-no-slide", state(x=0.35, flags=1|64), [(-4,-8,-4,0.4,1,4)], 1, [(1,0,0)]),
        ("support-gap", state(x=0.15,flags=1|128), [(-4,-8,-4,0.2,1,4),(0.7,-8,-4,4,1,4)], 1, [(1,0,0),(1,0,0)]),
        ("no-clip", state(flags=16|128), [floor,(1,1,-1,2,3,1)], 1, [(2,0,0)]),
        ("web-scale", state(flags=32|128), [floor], 1, [(1,-0.5,0)]),
        ("allow-slide-off", state(x=0.65, flags=1), [floor,(1,1,-1,2,3,1)], 1, [(1,0,0)]),
        ("negative-corner", state(x=-1.65,z=-1.65,flags=1|128), [(-4,-8,-4,4,1,4),(-1,1,-1,0,3,0)], 1, [(1,0,1)]),
        ("ice-floor", state(flags=1|128), [floor], 79, [(0,0,0)]),
        ("air-fall", state(y=4.0, velocity=(0,-0.3,0), flags=128), [], 0, [(0,-1,0)]),
        ("ordered-boxes", state(x=0.65, flags=1|128), [floor,(1,1,-1,1.5,3,1),(1.5,1,-1,2,2,1)], 1, [(1,0,0)]),
    ]
    for case_id, initial, boxes, surface, deltas in special:
        rows.append(scenario("move-"+case_id,"move",initial,boxes,surface,[move_frame(*d) for d in deltas]))

    rng = random.Random(17380117)
    for i in range(512):
        x=rng.choice((-12,-4,-1.65,0,0.65,4,12))+rng.uniform(-0.2,0.2)
        z=rng.choice((-12,-1.65,0,1.65,12))+rng.uniform(-0.2,0.2)
        grounded=i%3!=0
        surface=79 if i%7==0 else (1 if grounded else 0)
        colliders=[]
        if grounded: colliders.append((-16,-8,-16,16,1,16))
        if i%4==0:
            wall_x=math.floor(x+0.9); wall_z=math.floor(z)
            colliders.append((wall_x,1,wall_z-0.45,wall_x+0.5,2.1,wall_z+0.45))
        flag=(1 if grounded else 0)|(16 if i%23==0 else 0)|(32 if i%19==0 else 0)|(64 if i%11==0 else 0)|(128 if i%5 else 0)
        vel=tuple(rng.choice((0.0,0.05,-0.05,rng.uniform(-0.4,0.4))) for _ in range(3))
        initial=state(x=x,y=(1 if grounded else rng.uniform(1,8)),z=z,velocity=vel,
                      y_offset=rng.choice((0.0,0.0625)),y_size=rng.choice((0.0,0.02)),
                      step_height=rng.choice((0.0,0.5,0.75)),fall_distance=rng.uniform(0,3),
                      yaw=rng.choice((0,90,180,-90,45,-45)),flags=flag)
        frames=[move_frame(*(rng.choice((-1.0,-0.05,0.0,0.05,1.0,rng.uniform(-2,2))) for _ in range(3)))
                for _ in range(1+i%3)]
        rows.append(scenario(f"move-random-{i:04d}","move",initial,colliders,surface,frames))

    # Long real travel traces cover dry air, stone, ice, edge support, turns, stop/start,
    # diagonal input, sneak and jumps with native on-ground gating.
    for i in range(112):
        kind=i%4
        x=(-7.25 + (i%8)*1.75) if i%2 else (7.25-(i%8)*1.75)
        z=-6.5+(i%7)*2.0
        grounded=kind!=3
        surface=79 if kind==1 else (1 if grounded else 0)
        colliders=[(-16,-8,-16,16,1,16)] if grounded else []
        if kind==2:
            colliders=[(-3,-8,-3,0.45,1,3)]
            x=0.2
        flags=(1 if grounded else 0)|128
        yaw=(0,90,180,-90)[i%4]
        initial=state(x=x,y=1.0 if grounded else 3.0,z=z,velocity=(0,0,0),yaw=yaw,flags=flags)
        frames=[]
        for tick in range(64):
            if tick<6: strafe,forward=0.0,0.8
            elif tick<10: strafe,forward=0.0,0.0
            elif tick<18: strafe,forward=0.45,0.65
            elif tick<26: strafe,forward=-0.6,0.0
            elif tick<34: strafe,forward=0.0,1.0
            elif tick<42: strafe,forward=0.0,0.0
            elif tick<50: strafe,forward=-0.35,0.8
            else: strafe,forward=0.0,0.0
            frame_yaw=(yaw,0.0,90.0,180.0,-90.0,45.0,-45.0,270.0)[(tick//8+i)%8]
            jump=grounded and tick in (2,27,51)
            sneak=(kind==2 and tick<18) or (i%5==0 and 42<=tick<50)
            frames.append(travel_frame(strafe,forward,frame_yaw,jump,sneak))
        rows.append(scenario(f"travel-{i:03d}-{('stone','ice','ledge','air')[kind]}","travel",initial,colliders,surface,frames))

    data=("\n".join(rows)+"\n").encode("ascii")
    if len(data)>16*1024*1024 or len(rows)>4096:
        raise RuntimeError("generated corpus exceeds protocol limits")
    return data


def paths():
    config=tomllib.loads((ROOT/"config/local.toml").read_text())["paths"]
    jar=config.get("client_jar",str(Path(config["minecraft_reference"])/"client-b1.7.3.jar"))
    return (ROOT/jar).resolve(strict=True)


def expected_invocations(data, output):
    input_rows=[line.split("\t") for line in data.decode("ascii").splitlines()]
    output_rows=[line.split("\t") for line in output.decode("ascii").splitlines()]
    states={(row[0],int(row[1])):row[2] for row in output_rows}
    counts={}
    for row in input_rows:
        case,mode,initial,_,_,frames=row
        frame_list=frames.split(";")
        if mode=="move": counts["move"]=counts.get("move",0)+len(frame_list)
        else:
            counts["travel"]=counts.get("travel",0)+len(frame_list)
            flags=int(initial.split(",")[-1]); grounded=bool(flags&1)
            for i,frame in enumerate(frame_list):
                parts=frame.split(",")
                if parts[3]=="1" and grounded: counts["jump"]=counts.get("jump",0)+1
                flags=int(states[(case,i)].split(",")[-1]); grounded=bool(flags&1)
    return counts


def invalid_protocol_checks(classes, jar, valid_data):
    row=valid_data.splitlines()[0]
    f=row.decode("ascii").split("\t")
    base=valid_data.splitlines()[0]+b"\n"
    over_delta=b"invalid\tmove\t"+f[2].encode()+b"\t-\t0\t"+b",".join(raw64(v).encode() for v in (33,0,0))+b"\n"
    duplicate=base+base
    bad_surface=base.replace(b"\t1\t",b"\t2\t",1)
    non_ascii=base+b"\xff\n"
    fields=f.copy(); fields[0]="too-many-frames"; fields[5]=";".join([fields[5]]*257)
    too_many_frames=("\t".join(fields)+"\n").encode("ascii")
    fields=f.copy(); fields[0]="too-many-boxes"; fields[3]=":".join([box(-1,0,-1,1,2,1)]*129)
    too_many_boxes=("\t".join(fields)+"\n").encode("ascii")
    fields=f.copy(); fields[5]=";".join([fields[5]]*256)
    many_states_rows=[("case%04d\t"%i+"\t".join(fields[1:])) for i in range(391)]
    too_many_states=("\n".join(many_states_rows)+"\n").encode("ascii")
    many_scenarios_rows=[]
    for i in range(4097):
        one=f.copy(); one[0]=f"case{i:04d}"
        many_scenarios_rows.append("\t".join(one))
    too_many_scenarios=("\n".join(many_scenarios_rows)+"\n").encode("ascii")
    too_much_input=b" "*(16*1024*1024+1)
    cases=(base+b"broken-second-line\n",over_delta,duplicate,bad_surface,non_ascii,base[:-1],
           too_many_frames,too_many_boxes,too_many_states,too_many_scenarios,too_much_input)
    with tempfile.TemporaryDirectory(prefix="movement-invalid-") as tmp:
        for i,invalid in enumerate(cases):
            audit=Path(tmp)/f"audit-{i}.json"
            result=subprocess.run(["java","-cp",f"{classes}:{jar}","MovementOracle",str(jar),str(audit)],
                                  input=invalid,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=False,timeout=30)
            if result.returncode==0 or result.stdout or not result.stderr:
                raise RuntimeError(f"invalid protocol case {i} was not atomically rejected")
            if audit.exists() or Path(str(audit)+".registry.stdout").exists():
                raise RuntimeError(f"invalid protocol case {i} emitted an audit sidecar")


def oracle(data, jar, audit_path, classes):
    result=subprocess.run(["java","-cp",f"{classes}:{jar}","MovementOracle",str(jar),str(audit_path)],
                          input=data,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=False,timeout=120)
    if result.returncode:
        raise RuntimeError(result.stderr.decode("ascii",errors="replace"))
    audit=json.loads(audit_path.read_text())
    registry=Path(str(audit_path)+".registry.stdout").read_bytes()
    if audit.get("registry_stdout_bytes")!=len(registry) or audit.get("registry_stdout_sha256")!=sha(registry):
        raise RuntimeError("native registry stdout capture differs from runtime audit")
    return result.stdout,audit,registry


def run(write=False):
    data=corpus()
    jar=paths()
    if sha(jar.read_bytes())!=JAR_SHA: raise RuntimeError("unexpected original client JAR SHA-256")
    with zipfile.ZipFile(jar) as archive:
        actual={name:sha(archive.read(name+".class")) for name in CLASS_SHA}
    if actual!=CLASS_SHA: raise RuntimeError("original class hashes differ from pinned movement oracle")
    with tempfile.TemporaryDirectory(prefix="movement-oracle-") as tmp:
        classes=Path(tmp)/"classes"; classes.mkdir()
        subprocess.run(["javac","-cp",str(jar),"-d",str(classes),str(ROOT/"tools/oracle/src/MovementOracle.java")],check=True,timeout=60)
        audit1_path=Path(tmp)/"first.json"; audit2_path=Path(tmp)/"second.json"
        output,audit1,registry1=oracle(data,jar,audit1_path,classes)
        repeated,audit2,registry2=oracle(data,jar,audit2_path,classes)
        invalid_protocol_checks(classes,jar,data)
        source_sha=sha((ROOT/"tools/oracle/src/MovementOracle.java").read_bytes())
    if output!=repeated or audit1!=audit2 or registry1!=registry2:
        raise RuntimeError("independent native runs or registry captures differ")
    expected=expected_invocations(data,output)
    audit_counts=audit1.get("invocations")
    if audit_counts!=expected: raise RuntimeError(f"native invocation counts differ: {audit_counts} != {expected}")
    if audit1.get("schema_version")!=1 or audit1.get("suite")!="movement": raise RuntimeError("runtime audit schema mismatch")
    if audit1.get("jar_sha256")!=JAR_SHA or audit1.get("class_sha256")!=CLASS_SHA: raise RuntimeError("runtime native hash audit mismatch")
    if audit1.get("method_mapping")!=METHOD_MAPPING or audit1.get("field_mapping")!=FIELD_MAPPING: raise RuntimeError("native method/field mapping mismatch")
    if audit1.get("surfaces")!=SURFACES: raise RuntimeError("native surface slipperiness bits mismatch")
    if audit1.get("case_count")!=len(data.splitlines()) or audit1.get("state_count")!=len(output.splitlines()): raise RuntimeError("native audit counts mismatch")
    if audit1.get("state_count")<5000 or len(output.splitlines())>100000: raise RuntimeError("movement state count outside acceptance bounds")
    if audit1.get("native_world_constructors")!=0 or audit1.get("environment")!="synthetic-static-dry": raise RuntimeError("synthetic world audit mismatch")
    if not isinstance(audit1.get("collision_queries"),int) or audit1["collision_queries"]<0: raise RuntimeError("invalid collision query audit")
    if not audit1.get("code_sources") or set(audit1["code_sources"])!=set(CLASS_SHA): raise RuntimeError("native code source audit incomplete")
    audit=dict(audit1)
    audit.update(input_sha256=sha(data),output_sha256=sha(output),repeat_output_sha256=sha(repeated),
                 oracle_sha256=source_sha)
    if write:
        FIXTURE.mkdir(parents=True,exist_ok=True)
        (FIXTURE/"input.tsv").write_bytes(data)
        (FIXTURE/"reference.tsv").write_bytes(output)
        (FIXTURE/"registry.stdout").write_bytes(registry1)
        (FIXTURE/"audit.json").write_text(json.dumps(audit,indent=2)+"\n")
    else:
        for name,content in (("input.tsv",data),("reference.tsv",output),("registry.stdout",registry1)):
            if (FIXTURE/name).read_bytes()!=content: raise RuntimeError(f"committed {name} differs from live native oracle")
        if json.loads((FIXTURE/"audit.json").read_text())!=audit: raise RuntimeError("committed native audit differs")
    return {"status":"PASS","case_count":audit["case_count"],"state_count":audit["state_count"],
            "input_sha256":audit["input_sha256"],"output_sha256":audit["output_sha256"],
            "class_sha256":audit["class_sha256"],"invocations":audit["invocations"],
            "collision_queries":audit["collision_queries"]}


if __name__=="__main__":
    import argparse
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write",action="store_true")
    args=parser.parse_args()
    print(json.dumps(run(args.write),indent=2))
