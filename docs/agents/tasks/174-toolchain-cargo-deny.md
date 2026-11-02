# 174 · Toolchain pin and cargo-deny

Status: done · Area: N · Depends on: – · Observations: – · Touches: root

## Goal
Pin the stable toolchain and add cargo-deny (licences MIT/Apache-compatible, advisories, duplicates).

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)
- docs/roadmap/DECISIONS.md (D10)

## Design decisions (planner)
- `rust-toolchain.toml` pinned to the CI version; deny.toml; CI job. Update gate.sh to run cargo-deny when installed.

## Done when
- [x] CI runs cargo-deny; gate passes.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: no tests added (config only). `cargo deny check` passes (advisories, bans, licenses, sources ok; 20 duplicate-version warnings).
- Toolchain: `rust-toolchain.toml` pins 1.99.1 (clippy, rustfmt, minimal profile); workspace `rust-version = "1.99"`. All CI jobs use `dtolnay/rust-toolchain@1.99.1`; bump both together.
- deny.toml policy: licences allow MIT, Apache-2.0 (+LLVM-exception), BSD-2/3, ISC, Zlib, Unicode-3.0, MPL-2.0. Exceptions: `epaint_default_fonts` (OFL-1.1, Ubuntu-font-1.0: bundled egui font files) and `tiny-keccak` (CC0-1.0, public domain). Advisories: vulnerabilities deny; unmaintained (workspace crates only) and yanked warn. Bans: multiple versions warn, no skip list. Sources: crates.io only, no git.
- CI: new `deny` job (EmbarkStudios/cargo-deny-action@v2). gate.sh runs `cargo deny check` when installed, else prints a skip line.
- License: `license = "MIT OR Apache-2.0"` in workspace.package, inherited by every crate; LICENSE-MIT and LICENSE-APACHE added.
- Deviations: none.
- Files changed: rust-toolchain.toml, deny.toml, LICENSE-MIT, LICENSE-APACHE, Cargo.toml, all crate Cargo.toml, ci.yml, gate.sh, this brief, BACKLOG.md.
- Open questions: none.
- Gate: ends `GATE: all passed`.
