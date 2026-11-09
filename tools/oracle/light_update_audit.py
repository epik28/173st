#!/usr/bin/env python3
"""Verify the bounded original-client light-update capture."""
import argparse, hashlib, json, pathlib, shutil, subprocess, sys, tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
FIX = ROOT / "tests/fixtures/light_update"
JAR_SHA = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"

def sha(data): return hashlib.sha256(data).hexdigest()

def main():
    ap = argparse.ArgumentParser(); ap.add_argument("--jar", default=str(ROOT.parent / ".local/minecraft/client-b1.7.3.jar")); args = ap.parse_args()
    jar = pathlib.Path(args.jar).resolve()
    if not jar.is_file() or sha(jar.read_bytes()) != JAR_SHA:
        raise SystemExit("wrong or missing pinned client JAR")
    if not shutil.which("javac") or not shutil.which("java"):
        raise SystemExit("javac and java are required")
    expected = (FIX / "native.tsv").read_bytes()
    with tempfile.TemporaryDirectory(prefix="light-update-audit-") as d:
        classes = pathlib.Path(d) / "classes"; classes.mkdir()
        subprocess.run(["javac", "-cp", str(jar), "-d", str(classes), str(ROOT / "tools/oracle/src/LightUpdateOracle.java")], check=True, stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        outputs = []
        for n in (1, 2):
            audit = pathlib.Path(d) / f"audit-{n}.json"
            p = subprocess.run(["java", "-cp", f"{classes}{__import__('os').pathsep}{jar}", "LightUpdateOracle", str(jar), str(audit)], input=(FIX / "input.tsv").read_bytes(), stdout=subprocess.PIPE, stderr=subprocess.PIPE, check=True)
            outputs.append(p.stdout)
        clean = lambda b: b"".join(x.rstrip(b"\r\n") + b"\n" for x in b.splitlines() if b"\t" in x)
        if outputs[0] != outputs[1] or clean(outputs[0]) != clean(expected):
            raise SystemExit("independent native captures differ from fixture")
        audit = json.loads((FIX / "audit.json").read_text())
        if audit["jar_sha256"] != JAR_SHA or audit["output_sha256"] != sha(clean(expected)):
            raise SystemExit("audit hash mismatch")
    print(f"PASS: 2 native captures, {len(expected.splitlines())} rows, output sha256={sha(expected)}")

if __name__ == "__main__": main()
