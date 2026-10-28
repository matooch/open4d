#!/bin/sh
# The full pre-commit gate (CLAUDE.md): fmt, clippy -D warnings, all tests with the GPU
# required, the cleanroom scan and the golden-data check. Run from the repo root.
# Takes several minutes; prints a summary at the end. Exit code 0 only if all pass.
set -u
fail=0
cargo fmt --all --check || { echo "GATE: fmt failed (run: cargo fmt --all)"; fail=1; }
cargo clippy --workspace --all-targets -- -D warnings || { echo "GATE: clippy failed"; fail=1; }
log="${TMPDIR:-${TEMP:-/tmp}}/forge-gate-tests.log"
# GNU timeout is missing on macOS: use gtimeout if present, else run without a limit.
to=""; command -v timeout >/dev/null 2>&1 && to="timeout 900"; [ -z "$to" ] && command -v gtimeout >/dev/null 2>&1 && to="gtimeout 900"
FORGE_REQUIRE_GPU=1 $to cargo test --workspace > "$log" 2>&1 || { echo "GATE: tests failed (log: $log)"; grep -B2 -A12 "panicked" "$log" | head -60; fail=1; }
grep "^test result" "$log" | awk '{p+=$4; f+=$6} END {print "GATE: " p " tests passed, " f " failed"}'
cargo run -q -p forge-cleanroom || { echo "GATE: cleanroom failed"; fail=1; }
cargo run -q -p forge-parity-harness --bin golden -- check || { echo "GATE: golden check failed"; fail=1; }
[ "$fail" = 0 ] && echo "GATE: all passed" || echo "GATE: FAILED"
exit "$fail"
