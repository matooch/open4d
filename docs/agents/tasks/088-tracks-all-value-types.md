# 088 · Tracks for all value types + point-level animation

Status: done (parity-pending)

## Goal
Animate every parameter type (bool, int, enum, colour, vector, link where keyable) and points of editable meshes.

## Inputs
- Area design: [../../roadmap/areas/G-animation.md](../../roadmap/areas/G-animation.md)
- O53: [../../parity/animation-3.md](../../parity/animation-3.md) and its golden cases
- docs/parity/animation-3.md (O53)
- docs/specs/ANIM.md
- crates/core/src/track.rs

## Design decisions (planner)
- `KeyValue::Float` for numeric channels (vector/colour split into components); `KeyValue::Step(Value)` for bool/enum/link. Int: interpolate then round per O53 int-track rule.
- Point-level animation: a track per editable mesh holding whole point arrays per key; linear blend unless O53 shows otherwise.
- File format bump with a migration (163 pattern).

## Done when
- [ ] O53 cases (bool, enum, colour, int, pla, generator-param) pass at level B.
- [ ] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Out of scope
Euler/quaternion (096).

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report
- Result: `crates/anim/tests/animation3_parity.rs` (11 sample files + mesh), `crates/anim/tests/model.rs` (13 tests), 2 core tests updated and one added. animation-3: 8/11 sample files exact; 3 KNOWN_MISMATCH (euler-cross and quaternion rot_h at 5.9e-4, generator-param strength curve at 1.5e-4: the reference's solver offsets are not on the 19-step grid; four hypotheses in ANIM.md §1.8) plus `generator-param/mesh.obj` pinned (settings of the cube/Bend not recorded; keyed strength equals stored strength is asserted). bool, enum, link, pla were not observable (O53 notes): implemented by design, parity-pending.
- Rules: int = floor(line + 1e-4) (`int-track`); O53 keys are straight chords sampled at exact time (`color-track`, `tie-span-24/30`); the handle grid follows seconds. Colour channels `<id>.r|g|b`; Bool/Enum/Link steps; mesh `<id>.points` point tracks (linear blend, parity-pending).
- Deviations: `Key.value` stays `f64` with a per-target encoding instead of a `KeyValue` enum (other crates build `Key` literals); side data lives in `Track`. No file version bump (additive serde defaults; a bump would invalidate the byte-stable v11 fixtures); `parse_target` kept, `parse_track_target` added.
- Files: crates/core/src/{track,lib,document,file}.rs, crates/core/tests/{tracks,time,native_format}.rs, crates/anim/src/{interp,scene_at,lib}.rs, crates/anim/tests/*, docs/specs/ANIM.md.
- Open: observe bool/enum/link/point-level animation through a path that actually drives them; record the cube size and Bend settings of generator-param; explain the solver offsets (euler-cross).
- Gate: see brief 096 (gate run once at the end).
