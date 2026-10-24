# Implementer playbook

You implement **one task brief** (`docs/agents/tasks/NN-*.md`) at a time. Read this whole
file, then [CLAUDE.md](../../CLAUDE.md) and [CLEANROOM.md](../CLEANROOM.md), then the
brief, then every file the brief lists under *Inputs*. Work only on the branch the brief
names. Do not push, merge, or open PRs: the planner does that.

## 0. Hard rules (stop and report if a task would need to break one)

1. Clean room: never open, read or search the reference app's install folder, binaries,
   resource/preset/string/icon files or SDK headers. Your only sources are the files in
   this repo (observations, golden data, specs) and public literature. Never use the
   reference app's or its maker's names in code, crate names, comments or UI.
2. Never edit `tests/golden/**` or `docs/parity/<feature>.md` observation files. If the
   data looks wrong or insufficient, say so in your report (`status: blocked`).
3. Order of work: **spec first, then tests, then code** (CLAUDE.md rule 5).
4. Never loosen a tolerance, never special-case a test case or file name in production
   code, never delete a failing case. Unmatched cases go into the test's `UNSUPPORTED` or
   `KNOWN_MISMATCH` list with a reason in the spec (§5 below).
5. `cargo clippy -- -D warnings` and `cargo test` must pass before each commit; run
   `sh scripts/gate.sh` before the final commit of a task.
6. No `unsafe` without a `// SAFETY:` comment; deterministic math only.

## 1. Environment

- Windows, Git Bash. Use `sh`, `awk`, `sed`, `grep`; **there is no usable Python** (the
  `python3` on PATH is a store stub that hangs: never call it).
- For non-trivial numeric experiments write a throwaway Rust test (§3.3), never Python.
- Edit files with your editor/edit tool. Avoid `perl -pi` or `sed -i` for multi-line or
  code edits: delimiter characters (`|`, `/`) inside the text have silently corrupted files
  before. If you must, check the result with `git diff` immediately.
- The full test suite takes minutes and needs the GPU (`FORGE_REQUIRE_GPU=1`); use
  `scripts/gate.sh`, which sets everything up. For quick loops run one test file:
  `cargo test -p forge-generators --test generators_parity -- --nocapture`.

## 2. Codebase map (what goes where)

| Crate (`crates/<dir>`, package `forge-<dir>`) | Holds |
|---|---|
| `core` | scene graph (`Scene`, `NodeId`, `Node`), documents and transactions (`Document`, `Tx`, undo), param descriptors (`ParamDesc`, `ParamKind`, `Value`, `Unit`), registry (`NodeType`, `Category`, `Plugin`), transforms (`scene.local_transform(id).matrix()`) |
| `geometry` | `PolyMesh { points: Vec<DVec3>, polys: Vec<Vec<u32>> }`, `Edge`, half-edge (`HalfEdgeMesh::build`, `is_closed`), normals, subdivision, triangulation (`triangulate_poly`), booleans, splines |
| `generators` | every object that produces geometry: primitives (`src/<name>.rs`, `round.rs`), modeling generators (`modeling.rs`), splines (`splines.rs`), spline generators (`spline_gens.rs`), subdivision, boole. `lib.rs` has `types()` + `build()` for parametric primitives, `Primitives::register` for all node types, `evaluate(scene, id)` (dispatch by type) and `is_consumed` |
| `modifiers` | deformers (Phase 7): empty stub so far; the first brief sets it up |
| `parity-harness` | golden-case discovery (`discover_cases`, `case.manifest()`, `case.expected_mesh()`), `compare_meshes` (strict: same counts, identical face lists, positions within 1e-4), `compare_geometric`, `export_weld`, `parse_obj`, the `golden` CLI |
| `viewport`, `ui`, `apps/studio` | wgpu viewport, egui shell; new node types appear in the UI automatically via the registry |

Patterns to copy (read them before writing anything similar):
- A parametric node type: `crates/generators/src/round.rs` (`cylinder_type`,
  `build_cylinder`): params via `ParamDesc::{float,int,bool,enumeration,vec3,link}` with
  `.range()` / `.unit()`; read them with `Params { ty, values }` →
  `p.float("radius")`, `p.int(..)`, `p.bool(..)`, `p.enumeration(..)`.
  **Angle params are stored in radians** (`.unit(Unit::Angle)`); case files hold degrees
  (the parity tests convert).
- A generator with children as inputs: `crates/generators/src/modeling.rs`
  (`inputs_of`, `placed`, `evaluate_modeling`) plus its three hooks in `lib.rs`
  (register, `evaluate` dispatch, `is_consumed`).
- A parity test that rebuilds a scene from `[params]` / `[params.input*]`:
  `crates/generators/tests/generators_parity.rs` (`apply`, the `UNSUPPORTED`,
  `KNOWN_MISMATCH` and `CAP_GEOMETRIC` lists, `check_cap_geometric`).
- A parity test for plain primitives: `crates/generators/tests/parity.rs`.
- Specs: `docs/specs/PRIMITIVES2.md`, `GENERATORS2.md` (tone, citations, the
  **parity-pending** marker for rules seen in only one case).

## 3. The loop for one task

### 3.1 Read the data
For each case in the brief: read `case.toml` (what was set), the observation row
(notes), and the mesh:
```bash
sh scripts/obj_dump.sh tests/golden/<feature>/<case>/mesh.obj 40
cargo run -q -p forge-parity-harness --bin golden -- info tests/golden/<feature>/<case>/mesh.obj
```
Start from the simplest case (defaults, smallest segment counts). Compare it with the
undeformed input (e.g. build the input with our own generator in a scratch test, §3.3).

### 3.2 Derive, then confirm on every case
Form a hypothesis for (a) **positions** (the math), (b) **vertex order**, (c) **face
order and each face's starting corner** (the strict comparison checks exact face lists,
rotation included). Typical finds from earlier work:
- Vertex order usually follows "originals first, then new points" or a row/column walk;
  print indices next to coordinates and look for the walk.
- Fitted constants can be exact rationals (the circle handle 83/150): fit at several
  points; if every sample gives the same value to 7 digits, it is the rule.
- Floating-point noise in the reference (1e-14) can create near-duplicates the exporter
  keeps; compare through `export_weld` on both sides when inputs combine.
- The exporter welds exactly equal points across the whole file.
Write each rule down in your notes with the case that proves it. A rule seen in one case
only is **parity-pending** (still implement it, mark it in the spec).

### 3.3 Scratch tests for numeric experiments
```rust
// crates/<crate>/tests/zz_scratch.rs  — DELETE before committing
#[test]
fn scratch() {
    let src = std::fs::read_to_string("../../tests/golden/bend/strength/mesh.obj").unwrap();
    let exp = forge_parity_harness::parse_obj(&src).unwrap();
    for (i, p) in exp.positions.iter().enumerate().take(10) {
        println!("{i}: {p:?}");
    }
}
```
Run with `cargo test -p <package> --test zz_scratch -- --nocapture`. Delete the file when
done (`git status` must not show it at commit time).

### 3.4 Write the spec section
In the spec file the brief names (`docs/specs/<SPEC>.md`): params table (id, label,
kind, default, range: from the observation's table), then the rules, each citing the
case(s) that show it (`bend/strength`), with **parity-pending** where seen once, and an
explicit list of what returns `BuildError::Unsupported` (observed options you do not
derive). Keep sentences short; formulas in backticks.

### 3.5 Write or extend the parity test
Usually extend the test the brief names; add the feature to its feature list. Start
with every case of the feature expected to match; move a case to `UNSUPPORTED` only if
you return `Unsupported` for its options, or to `KNOWN_MISMATCH` (builds but differs) only
after §3.7. Both lists are asserted exactly, so a later fix shows up.

### 3.6 Implement
Copy the patterns in §2. Keep functions small and pure (`fn deform(mesh, params) ->
PolyMesh`); wire them into the registry/evaluation as the brief says. Add a unit test for
behaviour the golden cases can't see (e.g. a param combination that must stay valid, a
closed mesh staying closed).

### 3.7 Iterate; when to stop
Run the feature's parity test; read the first failure (it names the case, the face or
vertex index, and the error). Fix the rule, rerun. If a case still fails after **three
genuinely different hypotheses**, stop working on it: pin it in `KNOWN_MISMATCH`, describe
what you measured in the spec ("known mismatch: …"), and continue with the other cases.
The planner may get a better observation.

### 3.8 Finish
1. `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
2. Update `docs/parity/STATUS.md`: one row per feature (spec §, golden folder and case
   count, test file, result "N/M exact; …").
3. Delete scratch files. `git status` shows only intended files.
4. Commit on the brief's branch, message: one summary line, a blank line, what was
   derived and the match count, then the trailer line from the brief.
5. Fill the brief's `## Report` section (template there) and set `status: done`
   (or `blocked`); commit that too.

## 4. Pitfalls seen before

- Face lists must match exactly, including the starting corner of each face.
- Case files use degrees; params store radians (convert in tests via `Unit::Angle`).
- `[params.input]`, `[params.input2]`, …: inputs in child order; `pos` / `rot` keys are
  the node transform, not params.
- The default of a param in the app can differ from the label you'd expect; trust the
  observation table, and when it is marked uncertain, trust what the mesh shows.
- Clippy: `needless_range_loop` (iterate collections instead of indices),
  `too_many_arguments` (use a struct), `type_complexity` (add a type alias).
- Don't call a generator recursively without a cycle guard (see the instance chain in
  `modeling.rs`).

## 5. Unsupported vs known mismatch

- `Unsupported`: an observed option whose rules you did not derive (the code returns
  `Err(BuildError::Unsupported("…"))`, the test lists the case in `UNSUPPORTED`).
- Known mismatch: the code builds something plausible but the case does not match
  (the test lists it in `KNOWN_MISMATCH` and asserts it still fails).
Both need a sentence in the spec saying why.
