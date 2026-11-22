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

Done: 193, 195, 196, 049, 110, 106, 164 (merged after planner review).

Overnight queue (2026-11-22). Work top to bottom. A brief's own `Status:` line may still say
`waiting: Oxx`; **the BACKLOG row is authoritative** (those observations are in). If a brief turns
out blocked, mark it `blocked: <reason>` in the brief and BACKLOG, push, and go on to the next.

1. 164 follow-up (on hermes/164-gltf): import GLB skin weights unchanged with the skin's
   `normalise` off and re-check the two pinned skinned-arm poses (one hypothesis; see the review).
2. [168](tasks/168-usd.md) · USD `.usda` import/export (public text format; round-trip tests)
3. [066](tasks/066-cloner-object-mode.md) · Cloner Object mode: distributions, seeds, selections (O21 golden cases)
4. [071](tasks/071-shader-spline-pushapart-etc.md) · Shader, Spline, Push Apart, Inheritance, Volume, Group effectors (O22 golden cases)
5. [062](tasks/062-vertex-maps-from-fields.md) · Vertex maps from fields; restriction (O49 golden cases)
6. [078](tasks/078-clone-tags-cache.md) · Clone selection and weight tags, clone cache (O50)
7. [077](tasks/077-instance-over-time.md) · Instance-over-time generator (O50)
8. [075](tasks/075-clone-text.md) · Clone-toolset Text (O31, O48)
9. [092](tasks/092-speed-graph.md) · Speed graph mode (own design, parity-pending G5)
10. [097](tasks/097-keyframe-indicators.md) · Keyframe indicators on every parameter (own design, G4)
11. [122](tasks/122-dynamics-bake-ui.md) · Bake to keys and cache tags; dynamics UI
12. [167](tasks/167-alembic.md) · Alembic (Ogawa) import/export (O44 has an ABC export)

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
4. **Budget.** A rule gets three hypotheses. A failing test gets at most 20 tool calls of debugging.
   After that, pin the case (KNOWN_MISMATCH / UNSUPPORTED with the reason, per IMPLEMENTER.md §5) or,
   if the brief cannot be finished, set its Status to `blocked: <reason>` in the brief and BACKLOG,
   commit, push and move to the next brief.
5. Before finishing, check for observed data the brief must use: every observation the brief's
   header names (`Observations: O..`) has a form in `docs/parity/` and usually golden cases in
   `tests/golden/<feature>/`. A brief whose cases exist but are not read by a parity test is not
   done. Say in the Report which cases you read and their result.
6. Finish: `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`; fill
   the brief's Report; set BACKLOG to `done`; commit; push the branch; add one line to
   `/Users/bigman/forge-implementer/log.md` (brief, branch, result).

## 6. Never

- Never edit `tests/golden/**` or `docs/parity/*.md`, never loosen a tolerance, never special-case a
  case name in production code, never delete a failing case.
- Never use the reference app, its names, or anything from its install folder.
- Never leave a build or test running in the background when you stop.
- Never "fix" an unrelated failing test: write it in notes.md and the brief's Report.
