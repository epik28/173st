# EVIDENCE-001 — integrity of saved math/java evidence

Scope: strengthen saved-report validation for the accepted math and java suites.
No Rust changes, no oracle formulas or fixture changes. This adds verification,
not game capability; the completion estimate must not increase for this work.

## C01 — verifier hardening

Owned paths: `tools/parity/math_review.py`, `tools/parity/java_review.py`,
optional shared `tools/parity/evidence.py`, `tests/python/test_evidence.py`.
Require every expected build/run/version/git process, successful return code,
no error/timeout, saved stdout/stderr and agreement with log paths. Bind saved
Rust executable and compiled Java adapter classes to their recorded hashes
before running protocol checks. Bind clean revision metadata where present.
Check recorded mismatch count/status against actual comparison. Preserve source,
JAR and output checks. Refuse missing records, changed executable/adapter, false
return codes, missing logs and inconsistent output paths with useful errors.

Add real regression tests that mutate a valid report and its retained artifacts;
they must call the verifier, rather than duplicate its checks. Do not rerun the
expensive live corpus per mutation. A new live report for each affected suite
must pass saved verification, including malformed protocol tests. Shared helper
must be limited to the evidence contract, no broad framework rewrite.

## C02 — independent integration

Coordinator reviews exact C01 commit, runs relevant Python tests and workspace
checks, obtains clean math/java live reports and verifies both with `--protocol`.
Then record provenance, limitations and acceptance in progress/handoff/status.
