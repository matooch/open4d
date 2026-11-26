# 218 · Input tracks: record, replay, Input node

Status: ready · Area: P · Depends on: 217 · Observations: – (own design, D21) · Touches: input, anim, behaviour, nodes, ui

## Goal
A live performance becomes deterministic data: actions are recorded to input tracks, replayed
in timeline mode, editable in the dope sheet, and usable without code through an
`input.action` node. BEHAVIOURS.md §7.5, §7.6, §7.8.

Spec: [../../specs/BEHAVIOURS.md](../../specs/BEHAVIOURS.md) (own design, D21; no parity data). Where the spec says *(open)*, decide, write the rule into the spec, and say so in the Report.

## Steps
1. Input tracks in the document (sampled per step, held values, step keys for buttons), saved and loaded, shown and edited in the dope sheet.
2. Recording during a play run (headless is enough here; the editor's record button comes in 219), committed as one undo step.
3. Timeline mode reads actions from the tracks only.
4. The `input.action` node in the expression graph (NODES.md §4), reading the track or the live value.

## Done when
- [ ] A recorded headless run, replayed in timeline mode, gives exactly the live outputs (§14 [218]).
- [ ] An expression tag with `input.action` drives a param from a recorded track, with no script.
- [ ] Dope sheet editing of an input track changes the replayed result.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
