"""VIEW-001 exact native mapping runner and saved evidence producer."""

from datetime import datetime, timezone
import hashlib
import importlib.util
import json
from pathlib import Path
import platform
import shutil
import subprocess
import sys
import tempfile
import time
import tomllib

try:
    import run as parity
except ImportError:  # unittest imports this module through the tools namespace.
    from tools.parity import run as parity

ROOT = parity.ROOT
FIXTURE = Path("tests/fixtures/view/live.tsv")
ORACLE = Path("tools/oracle/src/ViewOracle.java")
PROBE = Path("crates/beta-viewer/examples/view_probe.rs")
JAR_SHA256 = "af1fa04b8006d3ef78c7e24f8de4aa56f439a74d7f314827529062d5bab6db4c"
TERRAIN_SHA256 = "9e32336c83732193a20d0f942ae8aa1186ae01231245b5e83659ce6e9d3511cf"


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def source_snapshot(root):
    paths = {FIXTURE, Path("tests/fixtures/view/scene.tsv"),
             Path("tests/fixtures/view/audit.json"), Path("tests/fixtures/view/provenance.md"),
             ORACLE, Path("tools/oracle/view_corpus.py"), Path("tools/oracle/world_corpus.py"),
             Path("tools/parity/run.py"), Path("tools/parity/view_run.py"),
             Path("tools/parity/view_review.py"), Path("tools/parity/verify_report.py"),
             Path("config/local.toml"), Path("Cargo.toml"), Path("Cargo.lock"),
             Path("rust-toolchain.toml"), PROBE}
    paths.update(p.relative_to(root) for p in (root / "tests/fixtures/view").glob("*"))
    paths.update(p.relative_to(root) for p in (root / "tools/parity").glob("*.py"))
    return {str(p): sha256((root / p).read_bytes()) if (root / p).is_file() else None
            for p in sorted(paths)}


def compare_view_outputs(reference, actual, _cases=None):
    """Compare complete native/probe byte streams and enforce the C01 wire format."""
    try:
        from tools.oracle import view_corpus
    except ModuleNotFoundError:
        oracle_dir = str(ROOT / "tools/oracle")
        if oracle_dir not in sys.path:
            sys.path.insert(0, oracle_dir)
        import view_corpus
    try:
        view_corpus.parse_stdout(reference)
        view_corpus.parse_stdout(actual)
    except (UnicodeError, ValueError) as error:
        return {"mismatch_count": 1, "first_mismatch": {"record": None, "error": str(error)}}
    left, right = reference.splitlines(keepends=True), actual.splitlines(keepends=True)
    first = next((i for i in range(max(len(left), len(right)))
                  if i >= len(left) or i >= len(right) or left[i] != right[i]), None)
    return {"mismatch_count": 0 if first is None else 1,
            "first_mismatch": None if first is None else {
                "record": first,
                "reference": left[first].decode("utf-8", "replace") if first < len(left) else None,
                "rust": right[first].decode("utf-8", "replace") if first < len(right) else None}}


def _load_corpus(root):
    try:
        from tools.oracle import view_corpus
        return view_corpus
    except ModuleNotFoundError:
        oracle_dir = str(root / "tools/oracle")
        if oracle_dir not in sys.path:
            sys.path.insert(0, oracle_dir)
        import view_corpus
        return view_corpus


def _record_process(directory, processes, label, command, timeout, cwd):
    result = parity.run_process(command, cwd=cwd, directory=directory, label=label, timeout=timeout)
    processes[label] = result
    if result["error"] or result["returncode"] != 0:
        raise parity.RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
    return Path(result["stdout"]).read_bytes()


def run_view(root=ROOT, timeout=120):
    corpus = _load_corpus(root)
    parent = parity.artifacts_root(root) / "parity/view"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "view", "scenario_id": "VIEW-001-fixed-5x5-scene",
              "repository_root": str(root.resolve()),
              "started_at": datetime.now(timezone.utc).isoformat(), "status": "error",
              "reason": "comparison not completed", "reference_side": "Original Beta 1.7.3 uu/cv block renderer mapping",
              "reference_jar_path": None, "reference_jar_sha256": None,
              "reference_class_sha256": {}, "terrain_path": None, "terrain_sha256": None,
              "scope": "live block material mappings and neighbor visibility; no native vertex or pixel capture",
              "comparison_policy": "exact ordered native TSV bytes; no float fields in this mapping corpus",
              "platform": platform.platform(), "runtime_versions": {}, "rust_revision": None,
              "rust_dirty": None, "rust_binary_sha256": None, "adapter_class_sha256": None,
              "source_hashes": {}, "source_hashes_after": {}, "source_world_hashes": {},
              "source_world_hashes_after": {}, "input_copy_hashes": {}, "input_copy_hashes_after": {},
              "scene_sha256": None, "expected_fixture_sha256": None, "case_count": 0,
              "mapping_case_count": 0, "neighbor_pair_count": 0,
              "native_cases": {"live": 0, "synthetic": 0},
              "mismatch_count": None, "first_mismatch": None, "duration_ms": None,
              "reference_output_sha256": None, "rust_output_sha256": None,
              "processes": {}, "log_paths": {"report": str(report_path)}}
    initial = None
    evidence = None
    input_dir = None
    jar = terrain = None
    started = time.monotonic()
    try:
        cfg = tomllib.loads((root / "config/local.toml").read_text())["paths"]
        jar = (root / cfg["client_jar"]).resolve(strict=True)
        if sha256(jar.read_bytes()) != JAR_SHA256:
            raise parity.RunError("configured client JAR hash differs from pinned Beta 1.7.3")
        terrain_value = cfg.get("terrain_png")
        terrain = (root / terrain_value).resolve() if terrain_value else (root / cfg["minecraft_reference"] / "terrain.png").resolve()
        # Extraction is performed by the shared C01 helper and verifies the pinned resource.
        checked_jar, terrain = corpus.source(root)
        if checked_jar != jar or sha256(terrain.read_bytes()) != TERRAIN_SHA256:
            raise parity.RunError("configured terrain resource differs from pinned original")
        report.update(reference_jar_path=str(jar), reference_jar_sha256=JAR_SHA256,
                      terrain_path=str(terrain), terrain_sha256=TERRAIN_SHA256)
        initial = source_snapshot(root)
        report["source_hashes"] = initial
        input_dir, evidence = corpus.world_corpus.snapshot(directory, root)
        (directory / "snapshot.json").replace(directory / "snapshot-evidence.json")
        report["source_world_hashes"] = evidence["source_before"]
        report["input_copy_hashes"] = evidence["copied_before"]
        report["log_paths"]["input_copy_dir"] = str(input_dir)

        expected_fixture = (root / FIXTURE).read_bytes()
        scene = (root / "tests/fixtures/view/scene.tsv").read_bytes()
        report["scene_sha256"] = sha256(scene)
        report["expected_fixture_sha256"] = sha256(expected_fixture)
        report["case_count"] = len([line for line in scene.splitlines() if line and not line.startswith(b"#")])
        (directory / "scene.tsv").write_bytes(scene)
        (directory / "expected-live.tsv").write_bytes(expected_fixture)
        report["log_paths"].update(input=str(directory / "scene.tsv"),
                                   expected_output=str(directory / "expected-live.tsv"),
                                   scene=str(directory / "scene.tsv"))

        for name, cmd in (("version-java", ["java", "-version"]), ("version-javac", ["javac", "-version"])):
            _record_process(directory, report["processes"], name, cmd, timeout, root)
        report["runtime_versions"] = {name: (Path(proc["stdout"]).read_bytes() + Path(proc["stderr"]).read_bytes()).decode("utf-8", "replace").strip()
                                      for name, proc in report["processes"].items()}

        for run_name in ("native-run", "native-repeat"):
            run_dir = directory / run_name
            run_dir.mkdir()
            output, _, sections, audit = corpus.run_native(run_dir, input_dir, jar, terrain, root, timeout)
            (directory / f"{run_name}.stdout.tsv").write_bytes(output.encode())
            (directory / f"{run_name}.audit.json").write_text(json.dumps(audit, indent=2) + "\n")
            class_file = run_dir / "classes/ViewOracle.class"
            if not class_file.is_file():
                raise parity.RunError("javac did not retain ViewOracle class")
            if run_name == "native-run":
                report["adapter_class_sha256"] = sha256(class_file.read_bytes())
                report["reference_class_sha256"] = {k: audit[k] for k in ("uu_sha256", "cv_sha256")}
                reference = output.encode()
                report["reference_output_sha256"] = sha256(reference)
                report["native_audit"] = audit
                report["mapping_case_count"] = len(sections["states"])
                report["neighbor_pair_count"] = len(sections["neighbors"])
                report["native_cases"] = {"live": len(sections["states"]), "synthetic": 0}
            else:
                if output.encode() != reference or audit != report["native_audit"]:
                    raise parity.RunError("repeated native oracle output/audit differs")
        report["log_paths"].update(reference_stdout=str(directory / "native-run.stdout.tsv"),
                                   reference_audit=str(directory / "native-run.audit.json"),
                                   repeat_stdout=str(directory / "native-repeat.stdout.tsv"),
                                   repeat_audit=str(directory / "native-repeat.audit.json"))
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust view_probe source is absent; original Java mapping oracle ran twice")
        else:
            # Probe command line is fixed by VIEW-001: marked input-copy dir and original atlas path.
            report["rust_revision"] = _record_process(directory, report["processes"], "git-revision",
                                                       ["git", "rev-parse", "HEAD"], timeout, root).decode().strip()
            dirty = _record_process(directory, report["processes"], "git-dirty",
                                    ["git", "status", "--porcelain"], timeout, root)
            report["rust_dirty"] = bool(dirty.strip())
            build = _record_process(directory, report["processes"], "rust-build",
                                    ["cargo", "build", "--locked", "--offline", "-p", "beta-viewer",
                                     "--example", "view_probe", "--message-format=json-render-diagnostics"], timeout, root)
            binaries = [Path(obj["executable"]) for line in build.splitlines()
                        if (obj := json.loads(line)).get("reason") == "compiler-artifact"
                        and obj.get("target", {}).get("name") == "view_probe" and obj.get("executable")]
            if len(binaries) != 1:
                raise parity.RunError("cargo did not identify exactly one view_probe executable")
            binary = directory / "view_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            rust_input = directory / "rust-input"
            rust_input.mkdir()
            for item in input_dir.iterdir():
                shutil.copyfile(item, rust_input / item.name)
            report["log_paths"]["rust_input_copy_dir"] = str(rust_input)
            (directory / "rust-run.stdout.tsv").write_bytes(b"")
            result = parity.run_process([str(binary), str(rust_input), str(terrain), str(directory / "scene.tsv")], cwd=root,
                                        directory=directory, label="rust-run", timeout=timeout)
            report["processes"]["rust-run"] = result
            actual = Path(result["stdout"]).read_bytes()
            report["rust_output_sha256"] = sha256(actual)
            report["log_paths"]["rust_stdout"] = result["stdout"]
            if result["error"] or result["returncode"] != 0:
                raise parity.RunError(f"rust-run: {result['error'] or 'exit ' + str(result['returncode'])}")
            comparison = compare_view_outputs(reference, actual)
            report.update(comparison, status="pass" if comparison["mismatch_count"] == 0 else "fail",
                          reason="all native mappings match exactly" if comparison["mismatch_count"] == 0 else "mapping output mismatch")

        world_corpus = corpus.world_corpus
        world_corpus.verify_snapshot(directory, evidence, root)
        report["source_world_hashes_after"] = evidence["source_after"]
        report["input_copy_hashes_after"] = evidence["copied_after"]
        if sha256(jar.read_bytes()) != JAR_SHA256 or sha256(terrain.read_bytes()) != TERRAIN_SHA256:
            raise parity.RunError("client JAR or terrain resource changed during execution")
    except (parity.RunError, OSError, ValueError, KeyError, TypeError, json.JSONDecodeError,
            subprocess.SubprocessError, RuntimeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        if initial is not None:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if after != initial:
                report.update(status="error", reason="tracked inputs changed during parity run")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path
