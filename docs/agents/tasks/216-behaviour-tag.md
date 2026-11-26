# 216 · Behaviour tag, lifecycle and timeline mode

Status: ready · Area: P · Depends on: 215 · Observations: – (own design, D21) · Touches: behaviour (new), luau, core, app, nodes, io

## Goal
Objects get behaviours with start/update and state that persists across frames. In timeline
mode they evaluate deterministically and scrub through a per-frame cache.
BEHAVIOURS.md §3, §4.1–§4.3, §5.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. `crates/behaviour` (`forge-behaviour`): the `Behaviour` trait (§4.4, so the trait exists from the start), the Luau adapter, `Ctx` and `Object` handles.
2. The `forge.behaviour` tag (§3), with exposed params from the script's `params` table as data-driven descriptors (keyable, saved, undoable). Old files load unchanged.
3. Evaluation slot and order (§5.1, §5.2), and the output layer (§5.3), with commands refused inside the lifecycle.
4. `BehaviourCache` with the DYNAMICS.md §5 rule (§5.4): forward stepping, cached reads when scrubbing back, a new run on a revision change. Render and `forge render` go through it.
5. Errors as tag diagnostics (§5.5). Examples: a spring follower and a counter.

## Done when
- [ ] The lifecycle, determinism and output-layer tests of §14 [216] pass, and pass 5 runs in a row.
- [ ] A rendered sequence with a behaviour is byte-identical across two runs.
- [ ] Save and load round trip, including exposed params and keyed exposed params.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
