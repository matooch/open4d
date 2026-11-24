# Unattended implementer run (local agent)

This is the playbook for a long-running implementer agent driven by a local model on the owner's
Mac. It adds rules for working for hours without a human, on a model with a small context.
**[IMPLEMENTER.md](IMPLEMENTER.md) applies in full** (hard rules §0, environment §1, codebase map
§2, method §3 onwards); where this file is stricter, follow this file.

## 1. Where you work

- Your own clone: `/Users/bigman/Github/open4d-impl`. **Never** touch
  `/Users/bigman/Github/open4d`: the observer agent works there.
- Your own workspace for notes and state: `/Users/bigman/forge-implementer/`.
- You never use the reference app or its bridge. Your agent has no "cinema4d" MCP server; if it
  does, stop and tell the owner.
- One branch per brief: `hermes/<NNN>-<short-name>`, made from the latest `origin/main`. Push it
  (`git push -u origin hermes/<NNN>-<short-name>`). Never push to `main`, never open or merge PRs,
  never rewrite pushed history: the planner reviews your branch and opens the PR.

## 2. Which brief

Work only on briefs in the queue below, top to bottom, skipping ones whose Status in
`docs/roadmap/BACKLOG.md` is not `ready` (someone else may have taken it). Set the row to
`in-progress (hermes)` in your first commit on the branch.

### Queue (the planner edits this list)

Done: 193, 195, 196, 049, 110, 106, 164 (+ follow-up), 066, 168, 071, 062, 078, 077, 075, 092,
097, 122, 167, 145, 099, 132, 131, 140, 141, 124, 134, 129, 155, 146, 040, 054, 170 (merged after planner review).
Also done by the planner (not yours): 165, 166 (FBX import/export).

Run 4 queue (2026-11-24). **Stack the branches:** start each brief from the previous brief's
branch, not from `origin/main`, unless the previous one is already merged into `main`
(`git branch -r --contains <its last commit>` lists `origin/main`). That way the briefs of a run
build on each other and the planner merges them without conflicts. Merge `origin/main` into your
current branch at the start of each brief as well.

1. [146](tasks/146-s8-rendering-acceptance.md) · S8 acceptance test and rendering docs
2. [040](tasks/040-metaball.md) · Metaball generator (uses the volume mesher, 083)
3. [054](tasks/054-smoothing-displacer-reduction-bevel.md) · Smoothing deformer, Displacer, Polygon Reduction, Bevel deformer
4. [170](tasks/170-camera-light-material-exchange.md) · Camera, light and material exchange across glTF/USD/Alembic
5. [197](tasks/197-shading-ir-wgsl.md) · R1: shading IR → WGSL codegen and shader cache (start of area R; read `docs/roadmap/areas/R-realtime.md` and D19 first)
6. [142](tasks/142-gpu-path-tracer.md) · GPU compute path tracer (wgpu): the compute fallback that area R tiers 2–3 build on
7. [198](tasks/198-realtime-raster.md) · R2: real-time raster PBR engine (`render-rt`); your M5 Pro is the Metal reference
8. [199](tasks/199-realtime-post.md) · R3: post effects and the viewport "Render" mode
9. [205](tasks/205-s4-mograph-acceptance.md) · S4 acceptance: motion graphics through the CLI
10. [206](tasks/206-s5-volume-acceptance.md) · S5 acceptance: volumes through the CLI

198 and 199 show `waiting` in BACKLOG only on briefs earlier in this queue (197, 198); on a
stacked branch those are done, so start them in order. GPU tests run on the Mac's Metal adapter
(M5 Pro); keep them skippable without an adapter so CI stays green.

**Handing a brief back.** When a brief's branch is pushed, add one line to
`/Users/bigman/forge-implementer/log.md`: `<brief> <branch> <done|blocked> <one-line result>`. The
owner passes it to the planner, who reviews the branch (diff, gate, rules), then either opens the PR
or writes what to fix in the brief's `## Review (planner)` section on `main`. Before you start the
next brief, check whether the previous one got a Review section; if so, fix that first on the same
branch and push again.

When the queue is done (or you must stop): make sure `git status --short` is empty, then for
every `hermes/*` branch of this run run `git fetch -q origin` and
`git log --oneline origin/<branch>..<branch>`; it must print nothing, otherwise push that branch
and check again. Then write `QUEUE FINISHED` to `/Users/bigman/forge-implementer/log.md`, with
one line per brief: `<brief> <branch> <short SHA on origin> <done|blocked> <result, gate>`, and
stop. Never write `QUEUE FINISHED` while a branch has unpushed commits.

## 3. How to work with a small context

- Read only: this file, IMPLEMENTER.md, CLAUDE.md, the brief, and the files the brief names. Find
  code with `grep -rn` and read ranges (`sed -n 'A,Bp'`), never whole large files.
- Never print whole diffs, test logs or build logs. Filter them: `2>&1 | grep -E "^error|^warning|panicked|FAILED|test result" | head -40`.
- Keep `/Users/bigman/forge-implementer/state.json` current:
  `{"brief": "193", "branch": "hermes/193-extrude-auto", "step": "tests", "notes": "..."}`.
  After a restart, read it first and resume.
- Write findings (hypotheses tried, numbers, what failed) to
  `/Users/bigman/forge-implementer/notes.md` as you go.

## 4. Editing files over ssh

- Prefer small, exact edits made with a short Python script (read the file, `replace(old, new, 1)`,
  assert the old text was found once, write it back). Check the result with `git diff --stat` and a
  `sed -n` of the changed range.
- For a new file, write it whole with a quoted heredoc (`<<'EOF'`).
- Do not use `sed -i` or `perl -pi` on code: delimiter characters have corrupted files before.

## 5. The loop per brief

1. `git fetch origin && git checkout -b hermes/<NNN>-<name> origin/main`.
2. Spec first (the spec file the brief names), then tests, then code (CLAUDE.md rule 5).
3. Fast loop: `cargo test -q -p <crate> --test <file>` and
   `cargo clippy -q -p <crate> --all-targets -- -D warnings`. Commit each working step (only your
   files, `git add <paths>`).
4. **Done vs blocked.** A brief whose code works and whose unmatched cases are pinned
   (KNOWN_MISMATCH / UNSUPPORTED with a reason) is **done**, written as
   `done (parity-pending: <what is pinned>)`. Use `blocked` only when the brief cannot be
   built or tested at all (missing data the brief requires, a dependency not done).
5. **Budget.** A rule gets three hypotheses. A failing test gets at most 20 tool calls of debugging.
   After that, pin the case (KNOWN_MISMATCH / UNSUPPORTED with the reason, per IMPLEMENTER.md §5) or,
   if the brief cannot be finished, set its Status to `blocked: <reason>` in the brief and BACKLOG,
   commit, push and move to the next brief.
6. Before finishing, check for observed data the brief must use: every observation the brief's
   header names (`Observations: O..`) has a form in `docs/parity/` and usually golden cases in
   `tests/golden/<feature>/`. A brief whose cases exist but are not read by a parity test is not
   done. Say in the Report which cases you read and their result.
7. Finish: `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`; fill
   the brief's Report; set BACKLOG to `done`; commit; push the branch; add one line to
   `/Users/bigman/forge-implementer/log.md` (brief, branch, result).

## 5a. Tests that cannot flake

Two run-3 tests passed on the Mac and failed in CI. Write tests so speed and parallelism cannot
change the result:
- **Background work in UI tests** (renders, IPR, previews): after the action that starts it,
  advance with `h.step()` in a polling loop with a deadline until the result is there; call
  `h.run()` only once nothing is running. `run()` waits for a settled UI and fails when a job
  keeps requesting repaints.
- **No shared mutable globals between tests** (`static` counters, caches). Use per-test state;
  if a global is unavoidable, hold one `static Mutex` in every test that touches it.
- Run a new test file at least 5 times in a row before committing; any failure is a bug.

## 6. Never

- Never edit `tests/golden/**` or `docs/parity/*.md`, never loosen a tolerance, never special-case a
  case name in production code, never delete a failing case.
- Never use the reference app, its names, or anything from its install folder.
- Never leave a build or test running in the background when you stop.
- Never "fix" an unrelated failing test: write it in notes.md and the brief's Report.
