# Reviewer playbook

You check what the observer recorded **before** anyone builds on it. You never drive the
reference app, and you never edit golden data or what an observer wrote in a form's
tables. You add a `## Review` section to the form, write follow-up forms, and change
statuses.

## Inputs
- `origin/observations`, merged into your work branch (PIPELINE.md §2).
- The forms that changed, and their golden folders.
- [../roadmap/BACKLOG.md](../roadmap/BACKLOG.md), for the briefs that wait on those forms.

## Checks per finished form

1. **Header:** status, app version, OS, observer and date are filled in. Rows marked
   `TODO` have a reason.
2. **Every case row has output on disk:**
   - a `tests/golden/<feature>/<case>/` folder with `case.toml` and either `mesh.obj` or
     `samples*.json`;
   - or the row says why it was skipped.

   Then run:
   ```bash
   cargo run -q -p forge-parity-harness --bin golden -- check
   ```
3. **The data means what the row says.** Use `golden info` on the case folder:
   - counts and bounding boxes are plausible for the parameters;
   - the changed parameter really differs from the defaults case (look for byte-identical
     files: `cmp` between cases). A case identical to its defaults case tests nothing:
     flag it.
4. **`case.toml` is complete:**
   - every parameter of the object is listed under our keys, in case-file units (degrees,
     percent as shown);
   - keys are lowercase with `_`;
   - nested inputs are under `[params.input]`, `[params.input2]`, …;
   - effectors are under `[params.effector]`, `[params.effector_2]`, ….
5. **Clean room:**
   - no probe scripts are committed;
   - no app symbol names or parameter IDs appear in keys;
   - no screenshots or renders of the app's UI.

   `cargo run -q -p forge-cleanroom` must pass.
6. **Determinism:** where the form asked for a repeat run (seeds, renders), check that the
   form says whether the two runs matched.

## Write the `## Review` section (at the end of the form)

```markdown
## Review
Reviewer: <session> · Date: <date> · Verdict: sufficient | sufficient with gaps | insufficient
- For brief NNN: <which cases cover which rule; what is missing>.
- Repeated/void cases: <list> (identical to <case>).
- Follow-up: O<id> in <form> (<n> cases), queued.
```

## Follow-ups (no back and forth)

When data is missing or a case tested nothing:
1. Write a short form `docs/parity/<feature>-followup-N.md`. Use the same method header as
   the original, and give **concrete values** that differ from the defaults. Read the
   defaults from the observer's own parameter tables.
2. Add a `todo` row to [../parity/QUEUE.md](../parity/QUEUE.md) above the `gui-only` rows.
3. If a case can only be done in the GUI, add it to
   [../parity/GUI-SESSIONS.md](../parity/GUI-SESSIONS.md) instead.

## Unlock briefs

In BACKLOG.md, a brief listed as `waiting: Oxx` becomes `ready` when:
- every form it waits on is reviewed `sufficient` or `sufficient with gaps`; and
- the gaps are covered by **parity-pending** markers the brief already allows.

If a gap would block the brief, leave it `waiting` and point the waiting text at your
follow-up form.

## Commit

One commit per reviewed form:
```
Review: <form> (<verdict>); follow-up O<id>; unlock NNN, MMM
```
Then hand over to the lead (PIPELINE.md §4), who opens the PR.
