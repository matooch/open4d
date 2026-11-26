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
097, 122, 167, 145, 099, 132, 131, 140, 141, 124, 134, 129, 155, 146, 040, 054, 170, 197, 205, 206, 142, 198, 199, 200, 211, 201, 202, 203, 204, 212 (merged after planner review). 207 was done by a planner subagent: skip it.
Also done by the planner (not yours): 165, 166 (FBX import/export).

Overnight queue (2026-11-24, run 8). **Stack the branches:** start each brief from the
previous brief's branch, not from `origin/main`, unless the previous one is already merged into
`main` (`git branch -r --contains <its last commit>` lists `origin/main`). That way the briefs of a
run build on each other and the planner merges them without conflicts. Merge `origin/main` into your
current branch at the start of each brief as well.

Already done: 146, 040, 054, 170, 197, 205, 206, 142, 198, 199 (Hermes); 207, 208 and the 142
follow-ups (Vulkan fix, shader-node materials in the path tracer) by planner subagents.

Run 10 queue (2026-11-26; use a strong model). Start from `origin/main` once the planner has
merged run 9 there (check `git branch -r --contains a96496a`), otherwise from
`hermes/212-vulkan-rt-backend` with `origin/main` merged in:

1. [214](tasks/214-realtime-performance.md) · Real-time performance: persistent GPU scene, then RT effect budgets; measure S10 on the M5 Pro after each step
2. [213](tasks/213-raster-hybrid-quality.md) · Tier 1/2 quality gaps (mat-glossy first)

Then the UI round from the owner's first hands-on use (D22, GUI-SESSIONS.md owner notes), one branch per brief:

- [223](tasks/223-nav-tool-keys.md), [224](tasks/224-keyframe-toggle.md), [225](tasks/225-single-mode-ui.md), [229](tasks/229-object-manager-visuals.md), [226](tasks/226-docking.md), [227](tasks/227-icons-command-layout.md), [228](tasks/228-mograph-top-level.md) (230 comes after 215)

Then area P (behaviours, BEHAVIOURS.md, D21), one branch per brief, each from the previous one:

3. [215](tasks/215-luau-runtime.md) · Luau runtime and builder scripts
4. [216](tasks/216-behaviour-tag.md) · Behaviour tag, lifecycle, timeline mode and cache
5. [217](tasks/217-input-system.md) · Input system (devices, input map, injection)
6. [218](tasks/218-input-tracks.md) · Input tracks and the Input node
7. [220](tasks/220-bulk-clone-api.md) · Bulk clone API, Luau effector, budgets
8. [221](tasks/221-rust-behaviours.md) · Rust behaviours
9. [219](tasks/219-play-mode.md) · Play mode in the editor
10. [222](tasks/222-s11-runtime.md) · `forge play` and S11
11. [230](tasks/230-script-lookup.md) · Script Manager lookup and context search

The planner may take some of 215–222 with a subagent; skip any brief whose BACKLOG row is not `ready`.

Briefs 198–204 show `waiting` in BACKLOG only on briefs earlier in this queue; on a stacked branch
those are done, so start them in order. GPU tests run on the Mac's Metal adapter (M5 Pro); keep them
skippable without an adapter so CI stays green, and share one device per test file behind a lock
(§5a). **Vulkan ray tracing (RTX 3090) is not on this machine:** implement the Metal path and the
compute fallback, write the Vulkan code behind the same `RtBackend` trait so it compiles everywhere,
and record "Vulkan RT run pending on the owner's RTX 3090" in the Report as `parity-pending`. That
is `done`, not `blocked`.

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

## 2b. Overnight: one session after another

**Every session must make progress.** A session that only re-runs the start checks, re-reads
files and writes "resume" notes is wasted (the loop stops after two of those). After the start
checks (keep them short: one `git status`, one review check), do the next step in `state.json` and
commit at least one piece of work before ending: a spec section, a test file, or code. If the step
is too big for the session, split it, do the first part and commit it. If you truly cannot
progress, write `QUEUE PAUSED: <reason>` instead of ending quietly.

The owner may run you in a loop (`scripts/hermes-loop.sh`): when a session ends, a fresh one starts
with the same prompt and resumes from `state.json`. So:
- When your context is getting full, finish the step you are on, commit, push, update
  `state.json` (brief, branch, next step) and `notes.md`, and end the session. Do not start a
  large new step you cannot finish; never leave uncommitted work.
- Write `QUEUE FINISHED` (with the push check above) only when every brief in the queue is done or
  blocked. Write `QUEUE PAUSED: <reason>` when you must stop for a reason a fresh session cannot fix
  (disk full after cleaning, a push that keeps failing, a planner decision needed); the loop stops
  on either line.

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
- **GPU shaders must build on every backend, not only Metal.** Your Mac translates WGSL to MSL;
  Linux and Windows (Vulkan) translate it to SPIR-V and DX12 to HLSL, and naga has bugs that only
  one backend hits (brief 142's path tracer crashed naga's SPIR-V writer: "Expression [..] is not
  cached!"). Add a plain unit test that translates every module your code builds through naga to
  SPIR-V, MSL and HLSL (`naga::front::wgsl`, `valid::Validator`, `back::{spv,msl,hlsl}`), so CI
  catches it without a GPU. Linux CI runs GPU tests on a software Vulkan driver
  (`FORGE_REQUIRE_GPU=1`), so they really execute there.

## 6. Never

- Never edit `tests/golden/**` or `docs/parity/*.md`, never loosen a tolerance, never special-case a
  case name in production code, never delete a failing case.
- Never use the reference app, its names, or anything from its install folder.
- Never leave a build or test running in the background when you stop.
- Never "fix" an unrelated failing test: write it in notes.md and the brief's Report.
