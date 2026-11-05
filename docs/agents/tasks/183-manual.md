# 183 · User manual and generated references (mdBook, GitHub Pages)

Status: done · Area: N · Depends on: 019 · Observations: – · Touches: docs

## Goal
Manual in docs/manual, command/CLI/RPC/MCP and Rhai references generated from the registry, published by CI.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Generation is a `forge docs` subcommand.

## Done when
- [x] Pages deploy from main.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: 4 tests added in `apps/forge/tests/docs.rs` (SUMMARY links resolve and cover the generated references; every manual page is listed and its local links resolve; book config; pages workflow deploys only from main, no secrets). No parity data involved.
- Manual: 10 chapters (introduction, getting started, scenes, primitives, mesh editing, deformers and fields, cloner, animation, files and recovery, plugins) plus the existing automation chapter and the five generated references, written from docs/specs.
- Deviations: mdBook is not required by the gate (not installed here); the workflow builds it with a pinned `cargo install mdbook`. `book.toml` sits in docs/manual with `src = "."` and builds into `target/book`. Workflow actions use major-version tags like the existing workflows.
- Files changed: docs/manual/*.md, book.toml, SUMMARY.md, .github/workflows/pages.yml, apps/forge/tests/docs.rs, BACKLOG.md, this brief.
- Open questions: Pages must be enabled in the repository settings (source: GitHub Actions) before the first deploy.
- Gate: see the commit; `GATE: all passed`.
