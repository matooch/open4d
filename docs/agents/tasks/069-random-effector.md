# 069 · Random effector (all modes)

Status: done · Area: E · Depends on: 068, 063 · Observations: O8, O11, O22 · Touches: clone-engine, noise

## Goal
Random effector with random, gaussian, noise, turbulence and indexed modes.

## Inputs
- Area design: [../../roadmap/areas/E-clone-toolset.md](../../roadmap/areas/E-clone-toolset.md)
- O8: [../../parity/cloner.md](../../parity/cloner.md) and its golden cases
- O11: [../../parity/followups-1.md](../../parity/followups-1.md) and its golden cases
- O22: [../../parity/effectors-3.md](../../parity/effectors-3.md) and its golden cases
- docs/parity/effectors-2.md, effectors-3.md (random cases)
- forge-noise (063)

## Design decisions (planner)
- Seeded per clone index. Three-hypothesis budget for the reference sequence; otherwise statistical comparison (range, mean, variance per channel).
- Noise/turbulence modes sample forge-noise at clone position × scale + time × speed; record whether the rule matches (noise basis may differ: statistical fallback).
- Indexed mode: deterministic per index; should match exactly if the index rule is observable.

## Done when
- [x] Every random case has a test: exact where possible, statistical with a documented reason otherwise.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: node `forge.effector.random` (spec MOGRAPH.md E11-E17) with Random Mode random, gaussian, noise, turbulence (sorted is `Unsupported`), Synchronized, Indexed, Seed, Space (global, uv), Animation Speed, noise Scale; the Plain parameter set is shared (one `common_type` builder with per-effector defaults, one `Plain::apply_weighted` body; Random only supplies the per-clone, per-channel weights). Tests: `crates/generators/tests/cloner_parity.rs::random_effector_cases_match_the_reference` (8 cases), `crates/clone-engine/tests/random.rs` (14), one app test. Parity: **exact part 8/8** (clone count, visibility, colour, weight, every channel the effector does not randomise at 1e-4, expanded-mesh vertex and face counts); **statistical part**: Random mode (5 cases: `grid-random`/`grid-random-seed` are the same data, `grid-random-seed-7`, `random-scale`, `grid-random-rot`) inside the uniform band, Gaussian 1/1, Noise 1/1 (smoothness), Indexed range only. `KNOWN_MISMATCH`: Indexed statistics (`random-indexed`: observed mean -0.555, variance 0.117 over 15 values, outside the uniform band; asserted to stay outside). The pinned counts of the earlier cloner test (19 / 35 / 12) and the 13 Plain cases are unchanged.
- Random sequence: three hypotheses, all failed (E15): (1) one sequential stream of 15 standard generators from the seed (first 20 000 outputs, any use order, three mappings), (2) per-clone re-seeding with six seed/index combinations, (3) counter-based hashes of (seed, clone, channel). So the values are statistical. Rules: value `v = min + r (max - min)`, per-axis independent draws, one draw for Uniform Scale (`random-scale`), `grid-random` offset = 50 v; Gaussian standard deviation 1/6 (variance 0.024 observed); noise feature size 300 cm (rms step 0.119 observed vs 0.115); Minimum -100 % and Maximum 100 % are the range of `v` in every transform mode (own design).
- Deviations: the Indexed rule and all draw layouts are own design (not observable); Gaussian and noise constants are fitted to 15 values; Remap is not applied twice; `Plain` gained a `uniform_scale` field; `ParamDesc` seed is an int, the test converts the case files' float seed.
- Parity-pending: Turbulence, Synchronized, Space UV, Animation Speed, Sorted, hidden Scale defaults, Absolute/Remap with random channels, the Indexed distribution, the channel decorrelation of the noise.
- Files changed: crates/clone-engine/{Cargo.toml,src/{random,plain,effector,lib,node}.rs,tests/random.rs}, Cargo.lock, crates/generators/tests/cloner_parity.rs, crates/app/tests/cloner.rs, docs/specs/MOGRAPH.md, docs/parity/STATUS.md, docs/roadmap/BACKLOG.md, this brief.
- Open data that would settle the pending rules (MOGRAPH.md E17): Random mode Count 50 with Seeds 12345, 0, 1 and Strength 50 %; Indexed Count 50, and Counts 5 and 8 (does clone 0 keep its value); Gaussian Count 50; Noise Count 20 at 10 cm steps with Scale 50 % / 200 %, Animation Speed 0 % at frame 30, Space UV; Turbulence; Synchronized; Sorted; Random with Rotation on R.H only.
- Gate (last lines): `GATE: 830 tests passed, 0 failed`; python `OK` (38 tests); `cleanroom: ok`; `ok: 471 cases`; `GATE: all passed`.
