"""Strict native ItemStack/main-inventory parity runner."""
from datetime import datetime, timezone
import hashlib
import json
import os
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
except ImportError:
    from tools.parity import run as parity

try:
    from tools.oracle import items_corpus
except ImportError:
    sys.path.insert(0, str(parity.ROOT / "tools/oracle"))
    import items_corpus

ROOT = parity.ROOT
FIXTURE = Path("tests/fixtures/items/native.tsv")
INPUT = Path("tests/fixtures/items/input.tsv")
AUDIT = Path("tests/fixtures/items/audit.json")
ORACLE = Path("tools/oracle/src/ItemsOracle.java")
CORPUS = Path("tools/oracle/items_corpus.py")
PROBE = Path("crates/beta-core/examples/items_probe.rs")
SUPPORTED_LIMITS = {1: 64, 3: 64, 332: 16, 267: 1}
MAX_INPUT_BYTES = 1_000_000
MAX_ROWS = 2048


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def source_snapshot(root):
    paths = {FIXTURE, INPUT, AUDIT, Path("tests/fixtures/items/provenance.md"),
             ORACLE, CORPUS, Path("tools/parity/run.py"),
             Path("tools/parity/items_run.py"), Path("tools/parity/items_review.py"),
             Path("config/local.toml"), Path("Cargo.toml"), Path("Cargo.lock"),
             Path("rust-toolchain.toml"), PROBE}
    paths.update(p.relative_to(root) for p in (root / "tests/fixtures/items").glob("*"))
    paths.update(p.relative_to(root) for p in (root / "tools/parity").glob("*.py"))
    return {str(path): sha256((root / path).read_bytes()) if (root / path).is_file() else None
            for path in sorted(paths)}


def _stack(token, native=False, allow_zero=False):
    if token == b"-":
        return token
    fields = token.split(b",")
    if len(fields) != (5 if native else 4):
        raise ValueError("stack must contain id,count,damage,limit and native name only in C01")
    try:
        item_id, count, damage, limit = map(int, fields[:4])
    except ValueError as error:
        raise ValueError("stack fields must be decimal integers") from error
    if (item_id not in SUPPORTED_LIMITS or count < (0 if allow_zero else 1) or count > SUPPORTED_LIMITS[item_id]
            or damage < 0 or damage > 32767 or limit != SUPPORTED_LIMITS[item_id]):
        raise ValueError("stack item/count/damage/verified-limit outside captured contract")
    if native and (not fields[4] or any(c < 0x21 or c > 0x7e for c in fields[4])):
        raise ValueError("native display name must be nonempty printable ASCII")
    return b",".join(fields[:4])


def _result(token, native=False):
    if not token:
        return token
    if token == b"none":
        return token
    parts = token.split(b";")
    normalized = []
    keys = set()
    for part in parts:
        pair = part.split(b"=", 1)
        if len(pair) != 2 or not pair[0].isalpha() or pair[0] in keys or not pair[1]:
            raise ValueError("result must be ordered unique key=value fields")
        key, value = pair
        keys.add(key)
        if value in (b"true", b"false"):
            checked = value
        elif b"," in value:
            checked = _stack(value, native=native, allow_zero=True)
        else:
            try:
                number = int(value)
            except ValueError as error:
                raise ValueError("result value must be boolean, decimal, or a typed stack") from error
            if number < -32768 or number > 32767:
                raise ValueError("result integer outside i16 bounds")
            checked = str(number).encode("ascii")
        normalized.append(key + b"=" + checked)
    return b";".join(normalized)


def _row(raw, native=False):
    fields = raw.split(b"\t")
    if len(fields) != 6:
        raise ValueError("item state row must have six tab-separated columns")
    scenario, step, result, selected, selected_stack, slots = fields
    if (not scenario or not step or any(c not in b"abcdefghijklmnopqrstuvwxyz0123456789_-" for c in scenario + step)):
        raise ValueError("invalid scenario/step identifier")
    try:
        selected_value = int(selected)
    except ValueError as error:
        raise ValueError("selected index must be a decimal integer") from error
    if selected_value < -1 or selected_value > 9:
        raise ValueError("selected index outside captured native behavior (-1..9)")
    stack = _stack(selected_stack, native=native)
    slot_values = slots.split(b"|")
    if len(slot_values) != 36:
        raise ValueError("inventory row must contain exactly 36 main slots")
    checked_slots = [_stack(value, native=native) for value in slot_values]
    if selected_value < 0 or selected_value > 8:
        if stack != b"-":
            raise ValueError("out-of-hotbar selected index must resolve to an empty stack")
    elif stack != checked_slots[selected_value]:
        raise ValueError("selected stack differs from selected main-inventory slot")
    return b"\t".join((scenario, step, _result(result, native=native), selected,
                        stack, b"|".join(checked_slots)))


def normalize_stream(raw, *, native=False):
    if len(raw) > MAX_INPUT_BYTES:
        raise ValueError("item state stream exceeds byte limit")
    lines = raw.splitlines(keepends=True)
    if not lines or len(lines) > MAX_ROWS:
        raise ValueError("item state stream is empty or exceeds row limit")
    output = []
    keys = []
    for line in lines:
        if not line.endswith(b"\n") or b"\r" in line:
            raise ValueError("item state rows require canonical LF termination")
        row = _row(line[:-1], native=native)
        key = tuple(row.split(b"\t")[:2])
        if keys and key == keys[-1]:
            raise ValueError("duplicate adjacent scenario/step state")
        keys.append(key)
        output.append(row + b"\n")
    if len(set(keys)) != len(keys):
        raise ValueError("duplicate scenario/step state")
    return b"".join(output), keys


def compare_items(reference, actual):
    try:
        expected, expected_keys = normalize_stream(reference, native=True)
        observed, observed_keys = normalize_stream(actual)
        left, right = expected.splitlines(keepends=True), observed.splitlines(keepends=True)
    except (ValueError, UnicodeError) as error:
        return {"mismatch_count": 1, "first_mismatch": {"record": None, "error": str(error)}}
    first = next((i for i in range(max(len(left), len(right)))
                  if i >= len(left) or i >= len(right) or left[i] != right[i]), None)
    if expected_keys != observed_keys and first is None:
        first = 0
    return {"mismatch_count": int(first is not None),
            "first_mismatch": None if first is None else {
                "record": first,
                "reference": left[first].decode("ascii", "replace") if first < len(left) else None,
                "rust": right[first].decode("ascii", "replace") if first < len(right) else None}}


def parse_input(raw):
    if not raw or len(raw) > MAX_INPUT_BYTES:
        raise ValueError("items input is empty or exceeds byte limit")
    rows = raw.splitlines(keepends=True)
    if not rows or len(rows) > 256:
        raise ValueError("items input exceeds scenario limit")
    scenarios = []
    for line in rows:
        if not line.endswith(b"\n") or b"\r" in line:
            raise ValueError("items input requires LF-terminated rows")
        if line.startswith(b"#"):
            continue
        fields = line[:-1].split(b"\t")
        if not fields or any(not f for f in fields):
            raise ValueError("items input contains empty field")
        scenario = fields[0]
        if any(c not in b"abcdefghijklmnopqrstuvwxyz0123456789_-" for c in scenario):
            raise ValueError("invalid items scenario ID")
        scenarios.append(scenario)
    if not scenarios or len(set(scenarios)) != len(scenarios):
        raise ValueError("duplicate items scenario")
    return scenarios


def _record(directory, report, label, command, timeout, root, input_data=None):
    result = parity.run_process(command, cwd=root, directory=directory, label=label,
                                timeout=timeout, input_data=input_data)
    report["processes"][label] = result
    if result["error"] or result["returncode"] != 0:
        raise parity.RunError(f"{label}: {result['error'] or 'exit ' + str(result['returncode'])}")
    return Path(result["stdout"]).read_bytes()


def run_items(root=ROOT, timeout=60):
    parent = parity.artifacts_root(root) / "parity/items"
    parent.mkdir(parents=True, exist_ok=True)
    directory = Path(tempfile.mkdtemp(prefix="run-", dir=parent))
    report_path = directory / "report.json"
    report = {"schema_version": 1, "suite": "items", "scenario_id": "ITEMS-001-main-inventory-native",
              "repository_root": str(root.resolve()), "started_at": datetime.now(timezone.utc).isoformat(),
              "status": "error", "reason": "comparison not completed",
              "reference_side": "Pinned Beta 1.7.3 iz/ix methods on isolated in-memory inventory",
              "reference_jar_path": None, "reference_jar_sha256": None,
              "reference_class_sha256": {}, "native_audit": {}, "rust_revision": None,
              "rust_dirty": None, "rust_binary_sha256": None, "source_hashes": {},
              "source_hashes_after": {}, "input_sha256": None, "expected_fixture_sha256": None,
              "reference_output_sha256": None, "rust_output_sha256": None,
              "case_count": 0, "row_count": 0, "mismatch_count": None,
              "first_mismatch": None, "duration_ms": None, "processes": {},
              "log_paths": {"report": str(report_path)}, "platform": platform.platform(),
              "comparison_policy": "ordered typed TSV; stack identity is id,count,damage,verified_limit; C01 display name is normalized away",
              "scope": "36 main slots/hotbar and captured stack operations; no GUI/crafting/save writes"}
    initial, jar = None, None
    started = time.monotonic()
    try:
        paths = tomllib.loads((root / "config/local.toml").read_text())["paths"]
        jar = (root / paths["client_jar"]).resolve(strict=True)
        if sha256(jar.read_bytes()) != items_corpus.PINNED_JAR:
            raise parity.RunError("configured client JAR SHA-256 differs from pinned Beta 1.7.3")
        report.update(reference_jar_path=str(jar), reference_jar_sha256=items_corpus.PINNED_JAR)
        initial = source_snapshot(root)
        report["source_hashes"] = initial
        input_data = (root / INPUT).read_bytes()
        expected = (root / FIXTURE).read_bytes()
        audit_expected = json.loads((root / AUDIT).read_text(encoding="utf-8"))
        scenarios = parse_input(input_data)
        normalized_expected, expected_keys = normalize_stream(expected, native=True)
        if len(expected_keys) > MAX_ROWS or set(key[0] for key in expected_keys) != set(scenarios):
            raise parity.RunError("C01 output scenarios differ from bounded frozen input")
        if (audit_expected.get("jar_sha256") != items_corpus.PINNED_JAR
                or audit_expected.get("input_sha256") != sha256(input_data)
                or audit_expected.get("output_sha256") != sha256(expected)
                or audit_expected.get("repeat_output_sha256") != sha256(expected)
                or audit_expected.get("rows") != len(expected_keys)):
            raise parity.RunError("C01 audit hashes/counts differ from committed input and fixture")
        report.update(input_sha256=sha256(input_data), expected_fixture_sha256=sha256(expected),
                      case_count=len(scenarios), row_count=len(expected_keys))
        report["log_paths"].update(input=str(root / INPUT), expected_output=str(root / FIXTURE))
        (directory / "input.tsv").write_bytes(input_data)
        (directory / "expected-native.tsv").write_bytes(expected)
        report["log_paths"].update(retained_input=str(directory / "input.tsv"),
                                   retained_expected=str(directory / "expected-native.tsv"))
        outputs, audits = [], []
        for name in ("native-run", "native-repeat"):
            class_dir = directory / name / "classes"
            class_dir.mkdir(parents=True)
            _record(directory, report, name + "-javac",
                    ["javac", "-cp", str(jar), "-d", str(class_dir), str(root / ORACLE)],
                    min(timeout, 40), root)
            output = _record(directory, report, name,
                             ["java", "-XX:-UsePerfData", "-Xmx512m", "-cp",
                              str(class_dir) + os.pathsep + str(jar), "ItemsOracle", str(jar)],
                             timeout, root, input_data)
            audit_bytes = Path(report["processes"][name]["stderr"]).read_bytes()
            if not audit_bytes:
                raise parity.RunError(f"{name}: native class/method audit missing on stderr")
            normalized, keys = normalize_stream(output, native=True)
            if normalized != normalized_expected or keys != expected_keys:
                raise parity.RunError(f"{name}: native output differs from committed C01 fixture")
            stdout_file = directory / f"{name}.native.tsv"
            stdout_file.write_bytes(output)
            report["log_paths"][name + "_output"] = str(stdout_file)
            audit_path = directory / f"{name}.audit.json"
            audit = json.loads(audit_bytes.decode("utf-8"))
            audit_path.write_text(json.dumps(audit, sort_keys=True, indent=2) + "\n", encoding="utf-8")
            report["log_paths"][name + "_audit"] = str(audit_path)
            if audit.get("jar_sha256") != items_corpus.PINNED_JAR or Path(audit.get("code_source", "")).resolve() != jar:
                raise parity.RunError(f"{name}: native class code source/JAR audit mismatch")
            outputs.append(output)
            audits.append(audit)
        runtime_audit_keys = ("bootstrap", "bootstrap_stdout_sha256", "classes", "code_source",
                              "direct_calls", "jar_sha256", "native_methods", "rows")
        if (outputs[0] != outputs[1] or audits[0] != audits[1]
                or any(audits[0].get(key) != audit_expected.get(key) for key in runtime_audit_keys)
                or sha256(outputs[0]) != audit_expected.get("output_sha256")
                or sha256(outputs[1]) != audit_expected.get("repeat_output_sha256")):
            raise parity.RunError("native repeats or committed C01 audit differ")
        report.update(reference_output_sha256=sha256(outputs[0]),
                      reference_class_sha256=audits[0]["classes"],
                      native_audit={key: audits[0][key] for key in
                                    ("bootstrap", "bootstrap_stdout_sha256", "direct_calls", "native_methods", "rows")}
                      | {"repeatable": True})
        if not (root / PROBE).is_file():
            report.update(status="not_implemented", reason="Rust items_probe source is absent; native C01 oracle ran twice")
        else:
            report["rust_revision"] = _record(directory, report, "git-revision",
                                                ["git", "rev-parse", "HEAD"], timeout, root).decode().strip()
            dirty = _record(directory, report, "git-dirty", ["git", "status", "--porcelain"], timeout, root)
            report["rust_dirty"] = bool(dirty.strip())
            build = _record(directory, report, "rust-build",
                            ["cargo", "build", "--locked", "--offline", "-p", "beta-core",
                             "--example", "items_probe", "--message-format=json-render-diagnostics"], timeout, root)
            binaries = [Path(obj["executable"]) for line in build.splitlines()
                        if (obj := json.loads(line)).get("reason") == "compiler-artifact"
                        and obj.get("target", {}).get("name") == "items_probe" and obj.get("executable")]
            if len(binaries) != 1:
                raise parity.RunError("cargo did not identify exactly one items_probe executable")
            binary = directory / "items_probe"
            shutil.copy2(binaries[0], binary)
            report["rust_binary_sha256"] = sha256(binary.read_bytes())
            report["log_paths"]["rust_binary"] = str(binary)
            rust = _record(directory, report, "rust-run", [str(binary)], timeout, root, input_data)
            report["rust_output_sha256"] = sha256(rust)
            report["log_paths"]["rust_output"] = report["processes"]["rust-run"]["stdout"]
            comparison = compare_items(outputs[0], rust)
            report.update(comparison, status="pass" if comparison["mismatch_count"] == 0 else "fail",
                          reason="all ordered inventory states match native C01 rows" if comparison["mismatch_count"] == 0
                          else "native inventory state mismatch")
        report["log_paths"].update(reference_stdout=report["processes"]["native-run"]["stdout"],
                                   repeat_stdout=report["processes"]["native-repeat"]["stdout"])
        if sha256(jar.read_bytes()) != items_corpus.PINNED_JAR:
            raise parity.RunError("client JAR changed during native items run")
    except (parity.RunError, OSError, ValueError, KeyError, TypeError,
            json.JSONDecodeError, subprocess.SubprocessError, RuntimeError) as error:
        report.update(status="error", reason=str(error))
    finally:
        if initial is not None:
            after = source_snapshot(root)
            report["source_hashes_after"] = after
            if after != initial:
                report.update(status="error", reason="tracked inputs/sources changed during items parity run")
        report["duration_ms"] = round((time.monotonic() - started) * 1000, 3)
        report_path.write_text(json.dumps(report, ensure_ascii=False, indent=2) + "\n")
    return report, report_path


def main(argv=None):
    import argparse
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--timeout", type=float, default=60)
    parser.add_argument("--protocol", action="store_true", help="verify the retained report after execution")
    args = parser.parse_args(argv)
    if not 0 < args.timeout < float("inf"):
        parser.error("--timeout must be finite and positive")
    report, path = run_items(timeout=args.timeout)
    print(f"{report['status']}: {report['reason']}\nreport: {path}")
    if args.protocol:
        try:
            from items_review import verify_report
        except ImportError:
            from tools.parity.items_review import verify_report
        verify_report(path)
        print("saved items report protocol: PASS")
    return parity.exit_code(report["status"], strict=True)


if __name__ == "__main__":
    raise SystemExit(main())
