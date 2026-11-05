# 177 · Signing and notarization hooks

Status: done (pending first signed run) · Area: N · Depends on: 176 · Observations: – · Touches: .github

## Goal
codesign/notarytool and signtool steps that run only when secrets exist.

## Inputs
- Area design: [../../roadmap/areas/N-platform.md](../../roadmap/areas/N-platform.md)

## Design decisions (planner)
- Skip cleanly without secrets.

## Done when
- [x] Workflow passes with and without secrets. (Structure verified by `apps/forge/tests/release_signing.rs`; real tools run only on a tagged run with secrets.)
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: `apps/forge/tests/release_signing.rs` (5 tests: signing steps gated, secrets only via env or `!= ''`, nothing echoed, `if: always()` cleanup, unsigned upload path, minimal permissions and pinned actions). Parity: n/a.
- Rules: gates `mac_gate` (sign, notarize), `win_gate`, `gpg_gate` are step outputs computed from `secrets.X != ''`; each skip logs `signing skipped: <reason>`. Secrets: MACOS_CERT_P12, MACOS_CERT_PASSWORD, MACOS_SIGN_IDENTITY (optional), MACOS_NOTARY_KEY_P8, MACOS_NOTARY_KEY_ID, MACOS_NOTARY_ISSUER_ID, WINDOWS_CERT_PFX, WINDOWS_CERT_PASSWORD, LINUX_GPG_PRIVATE_KEY, LINUX_GPG_PASSPHRASE (optional). Documented in docs/RELEASING.md.
- Deviations: the .app is signed inside `package-macos.sh` through `sign-hooks.sh` (it must be signed before the dmg exists); the workflow signs/notarizes/staples the dmg. Same for Windows exes via `sign-hooks.ps1`. No real signing tool was run; first signed tag run verifies it.
- Files changed: .github/workflows/release.yml, scripts/release/sign-hooks.sh/.ps1, package-macos.sh and package-windows.ps1 (comments), docs/RELEASING.md, apps/forge/tests/release_signing.rs, this brief, roadmap/BACKLOG.md.
- Open questions or data that would settle them (the reviewer turns these into follow-up forms).
- The last lines of the gate output.
- The last lines of the gate output: `GATE: all passed`.
