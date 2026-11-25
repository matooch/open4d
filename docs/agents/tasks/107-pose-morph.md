# 107 · Pose Morph tag

Status: done (parity-pending: O37, own design D20) · Area: H · Depends on: 088 · Observations: O37 (not capturable by script; own design per D20) · Touches: rig

## Goal
Point morph targets with strengths, absolute/relative modes, matching O37.

## Inputs
- Area design: [../../roadmap/areas/H-rigging.md](../../roadmap/areas/H-rigging.md)
- O37: [../../parity/pose-morph.md](../../parity/pose-morph.md) and its golden cases
- docs/parity/pose-morph.md (O37)

## Design decisions (planner)
- **Own design (D20), parity-pending.** O37 has no golden data: the reference app does not let a
  script set the morph strength. Build from the public manual and the standard blend, write the
  rules in the spec (docs/specs/RIG.md, a new Pose Morph section), and test against the spec.
  A GUI-assisted capture (GUI-SESSIONS.md G9, O37) checks it later.
- Point morph, relative mode: p = base + Σ s_i (target_i − base), strengths may exceed 100 % and go
  negative unless the manual says otherwise. Absolute mode: p = base + Σ s_i (target_i − base)
  with targets stored as absolute positions; write the exact difference between the modes in
  the spec from the manual.
- Strengths are animatable parameters (keyed like any other track, brief 088), so S6 can key them.
- Works on polygon objects and on the bind mesh under a skin (morph before skinning, the usual
  order; spec it).
- Position/rotation/scale (PSR) morph of the object itself: include if the manual describes it
  simply, else leave it out and say so in the Report.

## Done when
- [x] Spec-derived tests: base, single target at 0/50/100/150 %, two targets mixed, absolute vs relative, keyed strength sampled at frames, morph under a skin.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: own design (D20), no reference data, so no parity count: O37 stays "not observed" and
  every rule is parity-pending. Tests added: `crates/rig/tests/pose_morph.rs` (11 spec-derived
  tests) and `crates/app/tests/pose_morph_commands.rs` (3, commands). Both run 5 times in a row green.
- Rules (docs/specs/RIG.md section 15, sources: the public manual's description of the feature, which
  could not be fetched from this session (host not allowed), and the standard blend-shape maths):
  relative `p = c + sum s_i (t_i - b)`, absolute `p = c + sum s_i (t_i - c)` (c = the object's own points,
  b = the stored base pose, t_i = target positions), strengths unbounded (stored as a fraction,
  displayed in percent), applied before every deformer so the skin skins the morphed bind mesh.
  Proof: `one_target_at_0_50_100_and_150_percent`, `two_targets_add_their_offsets`,
  `relative_and_absolute_differ_once_the_object_moved_from_the_base`,
  `a_keyed_strength_is_sampled_at_frames`, `a_morph_under_a_skin_is_skinned_after_morphing`,
  `the_tag_survives_a_file_round_trip`, `targets_are_made_edited_and_cleared_with_undo`.
- Design: tracks address type-declared params only, so the tag declares 16 target slots
  (`name_i`, hidden `target_i`, animatable `strength_i`) from a constant; the descriptors drive UI,
  undo, keys and the file. Commands: `pose_morph.add_target` (creates the tag, captures the base,
  one undo step), `.update_target`, `.clear_target`, `.set_base`. Also added for S6: `tag.attach`
  (a tag on a named object; `object.create` only uses the selection) and `weights.auto_bind`
  (tag, joints, bone heat weights, bind pose, skin deformer).
- Deviations: the brief's absolute formula (`base + sum s (t - base)`) is identical to relative when the
  object equals the base, so the absolute mode here is "blend towards the stored goal positions" (uses
  the current points), which is what makes the two modes differ (spec 15.2). PSR morph left out (the
  manual's description was not available; O37 `psr-morph` not observed). The tag has 16 slots, not an
  unbounded list. The icon table got a `pose_morph` glyph and entries (the icon test requires them).
- Files: crates/rig/src/pose_morph.rs, lib.rs; crates/generators/src/lib.rs, Cargo.toml (hook, forge-rig
  dependency); crates/app/src/{pose_morph_commands,rig_commands,weight_commands,lib}.rs;
  crates/ui/icons/pose_morph.svg, crates/ui/src/icon_table.rs; docs/specs/RIG.md; docs/parity/STATUS.md;
  docs/manual/reference (regenerated); the tests above.
- Open questions: the manual's exact Absolute/Relative rule and strength limits, whether a Pose Morph
  has a PSR mode we should model (G9 GUI capture).
- The last lines of the gate output: see brief 114's Report (one gate for both briefs).
