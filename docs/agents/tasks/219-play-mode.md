# 219 · Play mode in the editor

Status: ready · Area: P · Depends on: 217, 218 · Observations: – (own design, D21) · Touches: ui, behaviour, viewport, app

## Goal
Press Play and the scene runs live, with a real clock, live devices, real-time rendering,
spawned objects and hot reload. Then stop, and the document is as it was. BEHAVIOURS.md §6, §8.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. Play, Stop and Record in the toolbar; the commands `play.start` / `play.stop` / `play.record`; input focus rules (§6).
2. The fixed-step accumulator with an injectable clock for tests (§6).
3. The runtime layer, `ctx.spawn` / `despawn`, the "Runtime" group in the Object Manager, `behaviour.bake` (§8).
4. Hot reload with `on_reload` (§6).
5. The script editor: the behaviour tag opens its script in a code panel (Luau highlighting, line numbers, errors at their line). Save triggers hot reload.
6. A perf overlay while playing: fps, step ms per behaviour, the behind counter.

## Done when
- [ ] The play tests of §14 [219] pass (fake clock, stop restores the frame and despawns, hot reload).
- [ ] The document's revision is unchanged by a play run without recording (test).
- [ ] An owner check in the editor: a gamepad moves an object live; record, stop, scrub, and the motion is there.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
