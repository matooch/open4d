# Unattended observer run (local agent + MCP bridge)

This is the playbook for a long-running observer agent on the owner's reference machine: a
local model driving the reference app through the MCP bridge from
[OBSERVER.md](OBSERVER.md). It adds rules for running for hours without a human, on a model
with a small context. **[OBSERVER.md](OBSERVER.md) §0 (clean-room rules) and §3 (the loop
per case) still apply in full**; where this file is stricter, follow this file.

## 1. What you may and may not touch

You **write only**:
- `tests/golden/<feature>/<case>/` — `case.toml`, `mesh.obj`, `samples.json`,
  `selection.json` (whatever the form asks for).
- `docs/parity/<form>.md` — fill in the case rows, Read back / Result / Notes columns, and the
  Notes section of the form you are working on.
- `docs/parity/QUEUE.md` — only the Status cell of the job you are working on.
- Your own workspace outside the repo: `~/forge-observer/` (probe scripts, logs, state).

You **never**:
- edit code, specs (`docs/specs/`), briefs, other forms, or other tests;
- edit an existing golden case (create a new case folder instead and say why in the form);
- commit probe scripts, app scene files, screenshots, renders of the app's UI, parameter IDs
  or symbol names (they stay in `~/forge-observer/`);
- invent, estimate or "fill in" a number. If you could not read it, write `not observed`
  and the reason;
- call the forbidden tools in [OBSERVER.md](OBSERVER.md) §0.

## 2. Start of every run

1. `cd` to the repo. `git fetch origin && git checkout observations` (create it from
   `origin/main` if it does not exist), then `git merge origin/main` so new forms and
   follow-up rows arrive.
2. Read `docs/parity/QUEUE.md`. Read `~/forge-observer/state.json` if it exists (§6) and
   resume the job and case it names.
3. `ping` the bridge. If it fails three times a minute apart, write the error to
   `~/forge-observer/log.md` and stop the run.

## 3. Which job next

Work through the queue in this order, skipping jobs whose Status is `done`, `blocked` or
`gui-only`:

1. **Follow-up cases** in forms whose Status reads `todo (N follow-up cases …)`. They sit in
   a section titled `## Follow-up cases (planner, …)` near the end of the form. Only those
   rows are new; the cases above them are already done.
2. **Forms never started**, in the order below. It is the order that unblocks the most
   implementation work:
   O29, O44, O32, O33, O34, O35, O36, O37, O27, O38, O53, O31, O50, O49, O28, O42, O43,
   O30, O57, O54, O39, O40, O41.
3. `partial` jobs (O25, O26): only the cases their Notes list as missing.

Set the job's Status to `in-progress` when you start it.

## 4. The loop per case

Do one case at a time and finish it before reading the next row.

1. **Fresh document.** New document, then build exactly what the row says. Create objects
   with the app's own menu command (OBSERVER.md §3.2), then set only the listed values.
2. **Read everything back** and write what the app shows. Clamped or rounded values are
   findings: note them in the row.
3. **Frame and time.** If the case involves animation, simulation, a deformer that keeps
   state, or anything named "frame", set the document frame explicitly **and write
   `frame = N` and `fps = N` into `case.toml`** under `[params]`.
4. **Check the result really happened before exporting.** Compare against the plain input:
   - For a deformer, cloner, effector or modifier case, the exported mesh must differ from
     the undeformed input (`golden info` sizes/bbox, or compare point positions). If it is
     identical, the scripted path did not evaluate it.
   - Then try once: initialise the object (its Initialize button, if it has one), step the
     document from frame 0 to the case frame one frame at a time, and export again.
   - If it is still undeformed, **do not commit the mesh**. Write
     `needs interactive export: scripted evaluation left it undeformed` in the row, and move on.
5. **Export / sample** to `tests/golden/<feature>/<case>/` exactly as the form says
   (OBJ settings: [../parity/export-settings.md](../parity/export-settings.md)).
6. **Write `case.toml`** (template: [../parity/README.md](../parity/README.md) §2). Rules
   that broke earlier batches:
   - **Every parameter of the object goes in**, not only the changed ones.
   - **No key may appear twice in one table.** A child object's kind is `type = "sphere"`;
     its own Type setting is `<object>_type = "standard"` (e.g. `sphere_type`).
   - Folder names: lowercase with dashes, exactly the case name in the form.
   - `feature` = the folder under `tests/golden/`, `observation` = the form's path.
7. **Run the checks** from the repo root:
   - `cargo run -q -p forge-parity-harness --bin golden -- info tests/golden/<feature>/<case>/mesh.obj`
     and paste its output into the row (mesh cases).
   - `cargo run -q -p forge-parity-harness --bin golden -- check` must print `ok: N cases`.
     If it reports your case, fix the case (never another one) and run it again.
8. **Big files.** A `mesh.obj` over 50 MB goes into Git LFS (`git lfs track` that one path,
   commit `.gitattributes` with it). Prefer smaller settings if the form allows a choice.
9. **Save state** (§6) and go to the next row.

A case that fails three times (tool error, crash, timeout) is marked
`not observed: <the error text>` in its row, and you go on. Do not retry it later in the
same run.

## 5. Commit, push, hand back

- After **every 5 cases** and at the end of each job: run `golden check`, then commit only
  your files with the message `Observation: <form title> (<job>, N cases)` and, if allowed,
  push (`git push origin observations`). If pushing is not allowed or fails, leave the
  commit local; the owner pushes.
- At the end of a job, set its QUEUE Status to `done`, `partial` (some cases missing, reason
  in the form's Notes) or `blocked` (nothing worked, reason in the Notes).
- Add one line per job to `~/forge-observer/log.md`: job, cases done, cases skipped with the
  reason, anything surprising.

## 6. Staying on track with a small context

- Keep `~/forge-observer/state.json` up to date after every case:
  `{"job": "O29", "form": "docs/parity/scene-defaults.md", "next_case": "camera-defaults", "done": [...], "skipped": {...}}`.
  After a restart or a context reset, read it first and resume there.
- Read only the form you are working on, this file, and OBSERVER.md §0 and §3. Do not read
  code or specs.
- Before each case, reread that case's row. Do not work from memory of earlier rows.
- When a row is unclear, do the most literal reading, write the reading you chose in the
  Notes column, and move on. Never ask and wait.

## 7. Things a human must do

Skip these in unattended runs. Leave them for the owner and say so in the form:
- jobs marked `gui-only`, and anything a form says belongs to a GUI session (G1–G8);
- cases that need a font picked in the GUI (O48's `dv-*` rows);
- cases that still exported undeformed after §4.4 (solver deformers, Spline Wrap).
