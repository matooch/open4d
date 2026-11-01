# N · Platforms, packaging, performance, docs

## Scope (v1)

- [ ] **CI matrix:** `ubuntu-latest`, `macos-latest` (arm64) and `windows-latest`.
  - fmt, clippy and tests on each;
  - GPU tests where a software adapter exists: lavapipe on Linux, WARP on Windows;
  - macOS GPU tests are allowed to skip when no adapter is found
    (`FORGE_REQUIRE_GPU` unset there).
- [ ] **Toolchain pin:** `rust-toolchain.toml` pins the stable version CI uses (the
  `fetch_update` incident). Updates to the pin are deliberate commits.
- [ ] **Supply chain:** `cargo-deny` for licences (DECISIONS D10), advisories and
  duplicate dependencies.
- [ ] **Cross-platform correctness:**
  - paths (`directories` crate for config, cache and data);
  - line endings (golden files normalised, as git already does);
  - fonts (bundled UI font);
  - HiDPI; keyboard modifiers (Cmd on macOS, Ctrl elsewhere: egui `COMMAND` already
    does this);
  - file dialogs (rfd: XDG portal, macOS, Windows).
- [ ] **Releases** (DECISIONS D11), from a tag:
  - Linux AppImage + tar.gz (bundle `libxkbcommon-x11`, see brief 05's note);
  - macOS universal2 `.app` in a `.dmg`;
  - Windows `.zip` + MSI;
  - checksums; a changelog generated from commits.
- [ ] **Signing hooks:** macOS codesign plus notarytool, Windows signtool. Each runs only
  when its secrets exist.
- [ ] **Crash handling:**
  - a panic hook writes a crash report (backtrace, last commands, versions) to the data
    directory, and the next start offers to open it;
  - structured logging (`tracing`) to a rotating file.
- [ ] **Autosave and recovery:** a periodic `.forge.autosave`; offered on start after a
  crash.
- [ ] **Performance:**
  - a dependency-graph evaluation scheduler (only dirty subgraphs, parallel with rayon);
  - profiling with `puffin` (an in-app profiler window in debug builds);
  - criterion benchmarks for generators, deformers, clone and volume pipelines;
  - S10 targets enforced as benchmark thresholds on Linux CI.
- [ ] **Fuzzing:** cargo-fuzz targets for the file readers (native, OBJ, glTF, FBX via
  ufbx, Alembic, VDB) and the command argument parser. A nightly CI job runs a short UI
  soak test (random commands through RPC for 10 minutes, asserting no panic).
- [ ] **Docs:**
  - a user manual (mdBook, `docs/manual/`);
  - a command/CLI/RPC/MCP reference generated from the registry;
  - a Rhai API reference;
  - published by CI to GitHub Pages.
- [ ] **Plugins:** Rhai plugin packages (a folder with a manifest that registers node
  types and commands written in Rhai). Native plugin ABI is v2.

## Observation jobs
None.

## Briefs

| # | Title | Depends on | Observations |
|---|---|---|---|
| 173 | CI matrix: Linux, macOS, Windows; per-OS GPU policy | – | – |
| 174 | Toolchain pin and cargo-deny | – | – |
| 175 | Cross-platform settings, paths, fonts, file dialogs, modifiers | 173 | – |
| 176 | Release workflow: AppImage, dmg, zip/MSI, checksums, changelog | 173 | – |
| 177 | Signing and notarization hooks | 176 | – |
| 178 | Crash handler and logging | 175 | – |
| 179 | Autosave and recovery | 175, 163 | – |
| 180 | Dependency-graph evaluation scheduler (parallel, dirty-only) | 012 | – |
| 181 | Benchmarks and S10 performance thresholds | 180, 144 | – |
| 182 | Fuzzing and UI soak test | 014, 163 | – |
| 183 | User manual and generated references (mdBook, GitHub Pages) | 019 | – |
| 184 | Rhai plugin packages | 016 | – |
| 185 | v1.0 release checklist: S1–S10 on all three OS | everything | – |

## Done when
A tagged release produces installable builds for all three OS. S1–S10 pass on each, and
the manual is published.
