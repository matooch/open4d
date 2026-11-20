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

1. [193](tasks/193-extrude-tests-use-auto.md) · Parity tests use Extrude's Auto direction (tests only; small)
2. [049](tasks/049-create-menu-organisation.md) · Create menu organisation (own design: every rule
   parity-pending until GUI session G1)

When the queue is done, write `QUEUE FINISHED` to `/Users/bigman/forge-implementer/log.md` and stop.

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
5. Finish: `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`; fill
   the brief's Report; set BACKLOG to `done`; commit; push the branch; add one line to
   `/Users/bigman/forge-implementer/log.md` (brief, branch, result).

## 6. Never

- Never edit `tests/golden/**` or `docs/parity/*.md`, never loosen a tolerance, never special-case a
  case name in production code, never delete a failing case.
- Never use the reference app, its names, or anything from its install folder.
- Never leave a build or test running in the background when you stop.
- Never "fix" an unrelated failing test: write it in notes.md and the brief's Report.
