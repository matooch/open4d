# The pipeline: observe → review → implement → integrate

Four roles, each driven only by files in this repository. No role waits for a person
except where a step is marked **owner**.

| Role | Runs where | Reads | Writes | Playbook |
|---|---|---|---|---|
| **Observer** | owner's Mac, with the reference app and its MCP bridge; unattended loop | [../parity/QUEUE.md](../parity/QUEUE.md) and the forms | form files and `tests/golden/**`, on branch `observations` | [OBSERVER.md](OBSERVER.md) |
| **Reviewer** | any session without the bridge (cloud or local) | finished forms and golden data | the form's `## Review` section, follow-up forms, brief statuses in [../roadmap/BACKLOG.md](../roadmap/BACKLOG.md) | [REVIEWER.md](REVIEWER.md) |
| **Implementer** | any session without the bridge | one brief plus its inputs | code, specs and tests; the brief's report | [IMPLEMENTER.md](IMPLEMENTER.md) |
| **Lead** | a top-tier session without the bridge (usually the cloud planner) | reports, diffs, gate output | merges, PRs, BACKLOG status, new briefs | this file, §4 |

The reviewer and the lead may be the same session. The observer must always be a separate
session, the only one with the bridge (clean room: [../CLEANROOM.md](../CLEANROOM.md)).

## 1. Observer loop (unattended)

1. `~/forge-observer/run-overnight.sh` (copy of [../../scripts/observer/run-overnight.sh](../../scripts/observer/run-overnight.sh)) pulls **`origin main`** into the local
   `observations` branch, then runs one fresh session per `todo` row of QUEUE.md.
2. Each job follows QUEUE.md rules 1–11:
   - check the bridge first;
   - one command per probe;
   - no dialogs;
   - push every 5 cases;
   - use the crash marker;
   - write a STOP file if the app is stuck.
3. The loop ends when the queue is empty, a STOP file appears, or a pull fails. **Owner:**
   restart the app or clear the STOP file when that happens, then start the loop again.

Branch: the observer commits and pushes only to `observations`.

## 2. Review (after each finished observation job)

Triggered when `observations` has commits that `main` doesn't. The reviewer:
1. Merges `origin/observations` into its work branch. Only `docs/parity/` and
   `tests/golden/` may change; reject anything else.
2. Runs `golden check`, the cleanroom scan and the harness tests. Updates the pinned
   counts in `crates/parity-harness/tests/samples.rs` and any pinned no-output list, but
   only where the data grew as the form intended.
3. For each finished form, writes a `## Review` section (REVIEWER.md):
   - is the data enough for each brief that waits on it;
   - which rows repeated defaults or failed;
   - which follow-up cases are needed.
4. Writes the follow-up cases as a new form and adds a `todo` row to QUEUE.md, so the
   observer picks them up on its next run without anyone asking.
5. Flips briefs in BACKLOG.md from `waiting: Oxx` to `ready` when their observations are
   reviewed and sufficient.

## 3. Implementation

1. An implementer takes the **lowest-numbered `ready` brief** in BACKLOG.md whose
   dependencies are all `done`. It sets that row to `in-progress` (and records which
   session took it) before starting.
2. It works the brief per IMPLEMENTER.md: spec → tests → code → full gate → report →
   commit on its work branch.
3. It sets the brief to `done` or `blocked` in the brief file and in BACKLOG.md.

Several implementers can run in parallel, as long as their briefs don't touch the same
crates. BACKLOG.md's "touches" column says which crates each brief touches.

## 4. Lead: integrate

1. Reviews every finished brief. For each, check:
   - **report:** it exists and is complete;
   - **spec:** it cites cases;
   - **tests:** they pin `UNSUPPORTED` and `KNOWN_MISMATCH`;
   - **no tolerance or golden edits.**
2. Reruns `sh scripts/gate.sh` on the same toolchain CI uses (`rustup update stable`
   first; see the brief 09 CI incident in PLAN.md).
3. Sends a brief back (`rework`, with notes) or accepts it.
4. Opens a PR to `main`, and enables auto-merge so it lands when CI is green. One PR per
   brief, or per batch if they are small.
5. When CI fails after a push, fixes it at once (CI red on `main` blocks everyone).
6. Every ~10 briefs: updates [../roadmap/README.md](../roadmap/README.md) milestones and
   [../parity/STATUS.md](../parity/STATUS.md).

## 5. Status vocabulary

| Where | Values |
|---|---|
| QUEUE.md row | `todo`, `in-progress`, `done`, `partial`, `blocked`, `gui-only` |
| BACKLOG.md row and brief header | `waiting: Oxx[, Oyy]`, `ready`, `in-progress (who)`, `done`, `blocked (why)`, `rework (notes)` |

## 6. What needs the owner

- **Merging PRs:** only if auto-merge is not enabled on the repository.
- **The observer machine:** keeping R25 open; restarting it after a crash or a STOP.
- **GUI-only observation rows (`gui-only`):** these need a person at the reference app.
  They are batched into [../parity/GUI-SESSIONS.md](../parity/GUI-SESSIONS.md), a list of
  short sittings with exact steps.
- **Licence confirmation** for scripted observation (`r25-drift.md` §0).
- **Release signing secrets**, if signed macOS/Windows builds are wanted (DECISIONS D11).
