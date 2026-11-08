#!/usr/bin/env python3
"""Generate geometry inputs and capture results from the pinned native client."""
import argparse
import hashlib
import json
import math
from pathlib import Path
import random
import subprocess
import tempfile
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[2]
FIXTURE = ROOT / "tests/fixtures/geometry"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
OPS = (
    "box_new", "box_sweep", "box_expand", "box_offset", "box_clip_x", "box_clip_y",
    "box_clip_z", "box_intersects", "box_contains", "box_average_edge", "box_ray",
    "vec_new", "vec_to", "vec_cross", "vec_add", "vec_normalize", "vec_length",
    "vec_distance", "vec_distance_sq", "vec_intermediate_x", "vec_intermediate_y",
    "vec_intermediate_z",
)


def sha(data):
    return hashlib.sha256(data).hexdigest()


def raw(x):
    return f"{int.from_bytes(__import__('struct').pack('>d', float(x)), 'big'):016x}"


def f(value):
    return raw(float(value))


def corpus():
    rows = []

    def add(case, op, *args):
        rows.append("\t".join((case, op, *(f(x) for x in args))))

    # Hand-picked boundaries, exact face contact, signed zero, degeneracy, inverted boxes,
    # near-threshold components, each ray face, edge/corner ties, and start-inside.
    eps = 1.0e-7
    edge_boxes = [
        (0, 0, 0, 1, 1, 1), (1, 0, 0, 2, 1, 1), (1-eps, 0, 0, 2, 1, 1),
        (1+eps, 0, 0, 2, 1, 1), (-2, -1, -3, -1, 0, -2),
        (0, 0, 0, 0, 1, 1), (2, 1, 1, -1, -1, -1),
        (-0.0, +0.0, -0.0, 1, 1, 1), (-30_000_000,)*3+(30_000_000,)*3,
    ]
    for i, b in enumerate(edge_boxes):
        add(f"edge-box-new-{i}", "box_new", *b)
        add(f"edge-box-avg-{i}", "box_average_edge", *b)
        add(f"edge-box-contains-{i}", "box_contains", *b, 0, 0, 0)
    base = (0, 0, 0, 1, 1, 1)
    contacts = [(1,0,0,2,1,1), (1-eps,0,0,2,1,1), (1+eps,0,0,2,1,1),
                (0.25,0.25,0.25,0.75,0.75,0.75), (-1,0,0,0,1,1)]
    for i, other in enumerate(contacts):
        add(f"edge-intersects-{i}", "box_intersects", *base, *other)
        for axis, delta in zip("xyz", (-1.0, 0.0, 1.0)):
            add(f"edge-clip-{i}-{axis}", f"box_clip_{axis}", *base, *other, delta)
    # Ray paths: six faces, two simultaneous-plane ties, and an origin within the box.
    ray_cases = [((-1,.5,.5),(2,.5,.5)), ((2,.5,.5),(-1,.5,.5)),
                 ((.5,-1,.5),(.5,2,.5)), ((.5,2,.5),(.5,-1,.5)),
                 ((.5,.5,-1),(.5,.5,2)), ((.5,.5,2),(.5,.5,-1)),
                 ((-1,-1,.5),(2,2,.5)), ((-1,-1,-1),(2,2,2)),
                 ((.5,.5,.5),(2,.5,.5)), ((-1,.3,.7),(2,.8,.2))]
    for i, (start, end) in enumerate(ray_cases):
        add(f"edge-ray-{i}", "box_ray", *base, *start, *end)
    # Very thin ray-axis extent with long opposing endpoints makes the native ray
    # candidates' squared distances nearly equal. Preserve both direction orders.
    near_tie_cases = [
        ("x", (0,0,0,1e-6,1,1), (1e6,.5,.5), (-1e6,.5,.5)),
        ("x", (0,0,0,1e-6,1,1), (-1e6,.5,.5), (1e6,.5,.5)),
        ("y", (0,0,0,1,1e-6,1), (.5,1e6,.5), (.5,-1e6,.5)),
        ("y", (0,0,0,1,1e-6,1), (.5,-1e6,.5), (.5,1e6,.5)),
        ("z", (0,0,0,1,1,1e-6), (.5,.5,1e6), (.5,.5,-1e6)),
        ("z", (0,0,0,1,1,1e-6), (.5,.5,-1e6), (.5,.5,1e6)),
    ]
    for axis, box, start, end in near_tie_cases:
        direction = "positive-negative" if start["xyz".index(axis)] > 0 else "negative-positive"
        add(f"ray-neartie-{axis}-{direction}", "box_ray", *box, *start, *end)
    # Vector corners and interpolation threshold behavior.
    vecs = [(0,0,0), (-0.0,0.0,-0.0), (1,2,3), (-1,2,-3), (eps,eps,eps),
            (1e-4,0,0), (1e-3,0,0), (30_000_000,-30_000_000,0)]
    for i, v in enumerate(vecs):
        add(f"edge-vec-new-{i}", "vec_new", *v)
        add(f"edge-vec-norm-{i}", "vec_normalize", *v)
        add(f"edge-vec-length-{i}", "vec_length", *v)
    for i, (a,b) in enumerate([((1,0,0),(0,1,0)), ((1,2,3),(-3,2,1)),
                                ((0,0,0),(0,0,0)), ((eps,0,0),(0,eps,0))]):
        add(f"edge-vec-to-{i}", "vec_to", *a, *b)
        add(f"edge-vec-cross-{i}", "vec_cross", *a, *b)
        add(f"edge-vec-dist-{i}", "vec_distance", *a, *b)
        add(f"edge-vec-distsq-{i}", "vec_distance_sq", *a, *b)
    for axis, op in enumerate(("vec_intermediate_x", "vec_intermediate_y", "vec_intermediate_z")):
        for i, plane in enumerate((0, eps, 0.5, 1, -1)):
            start=[-1.0,-1.0,-1.0]; end=[1.0,1.0,1.0]
            add(f"edge-intermediate-{axis}-{i}",op,*start,*end,plane)
    # Native bt interpolation compares squared axis delta with this literal; normalization
    # compares MathHelper's f32 sqrt result with 1e-4.
    interp_limit=1.0000000116860974e-7
    axis_limit=math.sqrt(interp_limit)
    for i, delta in enumerate((math.nextafter(axis_limit,0.0),axis_limit,math.nextafter(axis_limit,math.inf))):
        add(f"edge-interpolation-threshold-{i}","vec_intermediate_x",0,0,0,delta,1,1,delta/2)
    for i, length in enumerate((math.nextafter(1e-4,0.0),1e-4,math.nextafter(1e-4,math.inf))):
        add(f"edge-normalize-threshold-{i}","vec_normalize",length,0,0)

    # Large deterministic randomized domain, including inverted and flat boxes.
    rng = random.Random(1733173)
    for i in range(280):
        center = [rng.uniform(-30_000, 30_000) for _ in range(3)]
        ext = [rng.choice([0.0, rng.uniform(1e-7, 10)]) for _ in range(3)]
        box = tuple(center) + tuple(center[k] + ext[k] for k in range(3))
        other = tuple(center[k] + rng.uniform(-3,3) for k in range(3)) + tuple(center[k] + rng.uniform(-3,3) for k in range(3))
        delta = [rng.uniform(-5,5) for _ in range(3)]
        point = [rng.uniform(-30_000,30_000) for _ in range(3)]
        end = [center[k] + rng.uniform(-10,10) for k in range(3)]
        start = [center[k] + rng.uniform(-10,10) for k in range(3)]
        add(f"random-{i}-new", "box_new", *box)
        for op in ("box_sweep", "box_expand", "box_offset"):
            add(f"random-{i}-{op}", op, *box, *delta)
        for axis, op in enumerate(("box_clip_x","box_clip_y","box_clip_z")):
            add(f"random-{i}-{op}",op,*box,*other,delta[axis])
        add(f"random-{i}-intersects","box_intersects",*box,*other)
        add(f"random-{i}-contains","box_contains",*box,*point)
        add(f"random-{i}-avg","box_average_edge",*box)
        add(f"random-{i}-ray","box_ray",*box,*start,*end)
        add(f"random-{i}-vnew","vec_new",*point)
        add(f"random-{i}-vto","vec_to",*point,*end)
        add(f"random-{i}-vcross","vec_cross",*point,*end)
        add(f"random-{i}-vadd","vec_add",*point,*delta)
        add(f"random-{i}-vnorm","vec_normalize",*point)
        add(f"random-{i}-vlength","vec_length",*point)
        add(f"random-{i}-vdist","vec_distance",*point,*end)
        add(f"random-{i}-vdistsq","vec_distance_sq",*point,*end)
        for axis, op in enumerate(("vec_intermediate_x","vec_intermediate_y","vec_intermediate_z")):
            add(f"random-{i}-vi{axis}",op,*start,*end,center[axis])
    data = ("\n".join(rows) + "\n").encode("ascii")
    return data


def paths():
    config = tomllib.loads((ROOT / "config/local.toml").read_text())["paths"]
    jar_path = config.get("client_jar", str(Path(config["minecraft_reference"]) / "client-b1.7.3.jar"))
    return (ROOT / jar_path).resolve(strict=True)


def oracle(data, jar, audit_path):
    with tempfile.TemporaryDirectory(prefix="geometry-oracle-") as tmp:
        classes = Path(tmp) / "classes"
        classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes),
                        str(ROOT / "tools/oracle/src/GeometryOracle.java")], check=True, timeout=30)
        result = subprocess.run(["java", "-cp", f"{classes}:{jar}", "GeometryOracle", str(jar), str(audit_path)],
                                input=data, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
                                check=False, timeout=90)
        if result.returncode:
            raise RuntimeError(result.stderr.decode("ascii", errors="replace"))
        command=["java","-cp",f"{classes}:{jar}","GeometryOracle",str(jar),str(Path(tmp)/"invalid-audit.json")]
        valid="ok\tbox_new\t"+"\t".join(["0000000000000000"]*6)+"\n"
        invalid_inputs=[
            (valid+"bad\tunknown\t0000000000000000\n").encode("ascii"),
            (valid+valid).encode("ascii"),
            (valid+"bad\tvec_new\t7ff0000000000000\t0000000000000000\t0000000000000000\n").encode("ascii"),
            valid.encode("ascii")[:-1],
        ]
        for invalid in invalid_inputs:
            rejected=subprocess.run(command,input=invalid,stdout=subprocess.PIPE,stderr=subprocess.PIPE,check=False,timeout=20)
            if rejected.returncode==0 or rejected.stdout or not rejected.stderr:
                raise RuntimeError("malformed protocol was not atomically rejected")
        return result.stdout


def run(write=False):
    data = corpus()
    jar = paths()
    if sha(jar.read_bytes()) != JAR_SHA:
        raise RuntimeError("unexpected original client JAR SHA-256")
    with zipfile.ZipFile(jar) as archive:
        class_data = {name: archive.read(name + ".class") for name in ("eq", "bt", "vf", "in")}
    with tempfile.TemporaryDirectory(prefix="geometry-audit-") as tmp:
        first_audit, second_audit = Path(tmp)/"first.json", Path(tmp)/"second.json"
        output = oracle(data, jar, first_audit)
        repeated = oracle(data, jar, second_audit)
        audit1, audit2 = json.loads(first_audit.read_text()), json.loads(second_audit.read_text())
    if output != repeated or audit1 != audit2:
        raise RuntimeError("independent native oracle runs differ")
    expected_counts = {op: sum(row.split("\t",2)[1] == op for row in data.decode("ascii").splitlines()) for op in OPS}
    if audit1["invocations"] != expected_counts or audit1["case_count"] != len(data.splitlines()):
        raise RuntimeError("native invocation counts do not match cases")
    if len(data) > 32*1024*1024 or len(data.splitlines()) < 5000:
        raise RuntimeError("corpus size requirement failed")
    audit = {"schema_version":1,"suite":"geometry","jar_sha256":JAR_SHA,
             "class_sha256":{name:sha(blob) for name,blob in class_data.items()},
             "method_mapping":audit1["method_mapping"],"invocations":audit1["invocations"],
             "case_count":audit1["case_count"],"input_sha256":sha(data),
             "output_sha256":sha(output),"repeat_output_sha256":sha(repeated),
             "oracle_sha256":sha((ROOT/"tools/oracle/src/GeometryOracle.java").read_bytes())}
    if write:
        FIXTURE.mkdir(parents=True,exist_ok=True)
        (FIXTURE/"input.tsv").write_bytes(data)
        (FIXTURE/"reference.tsv").write_bytes(output)
        (FIXTURE/"audit.json").write_text(json.dumps(audit,indent=2)+"\n")
    else:
        for name,content in (("input.tsv",data),("reference.tsv",output)):
            if (FIXTURE/name).read_bytes()!=content: raise RuntimeError(f"committed {name} differs from live native oracle")
        if json.loads((FIXTURE/"audit.json").read_text())!=audit: raise RuntimeError("committed native audit differs")
    return {"status":"PASS","case_count":audit["case_count"],"input_sha256":sha(data),
            "output_sha256":sha(output),"class_sha256":audit["class_sha256"],
            "invocations":audit["invocations"]}


def main():
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--write",action="store_true")
    args=parser.parse_args()
    print(json.dumps(run(args.write),indent=2))


if __name__ == "__main__": main()
