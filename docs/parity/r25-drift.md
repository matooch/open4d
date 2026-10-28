# R25 drift check — observation (O1)

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Bridge: MCP plugin version TODO · Unblocks: trust in the R20 golden data for the R25
target; the R25 exporter settings.

Every golden case before batch 08 came from R20 on Windows. This form re-exports a sample
of them in R25 (scripted, [agents/OBSERVER.md](../agents/OBSERVER.md)) and records whether
each file is identical. Nothing in `tests/golden/<feature>/` is changed by this form.

## 0. Smoke test (O0, OBSERVER.md §2)

| Step | Result (ok / error text) |
|---|---|
| EULA checked by the owner (OBSERVER.md §1 step 6) | |
| plugin listening (`lsof` on port 5555), token log line seen | |
| `ping` | |
| `get_capabilities`: app version string, plugin version | |
| `execute_python_script` returns version and `1 + 1` | |
| probe: default cube → OBJ export | |
| probe: key position X at 0 and 30, read the value at 15 | |

## 1. R25 OBJ exporter options

List every option of the R25 OBJ export dialog (or what the probe can set), as in
[export-settings.md](export-settings.md). Mark options that are new since R20 and
defaults that changed. Use the R20 values for every option R20 had; for new options
write what you chose and why.

| Option label | Value used | Default | New since R20? | Set from probe how (plain words) |
|---|---|---|---|---|
| TODO | | | | |

## 2. Cases

For each case: rebuild it from its `case.toml` in R25, export with §1's settings to
`tests/golden/r25-drift/<feature>-<case>/mesh.obj`, then compare with the R20 file:
`cmp old new` (identical?) and `golden info` on both. **If identical, delete the R25
export** (don't commit duplicates). If different, keep it, write a `case.toml` copied
from the original with `feature = "r25-drift"`, `observation = "docs/parity/r25-drift.md"`,
and describe the difference (counts, bbox, a few differing lines).

| R20 case | Identical? | If not: counts old → new, bbox old → new, what differs |
|---|---|---|
| calibration/offset-cube | | |
| cube/defaults | | |
| cube/fillet | | |
| sphere/defaults | | |
| sphere/type-icosahedron | | |
| cylinder/fillet | | |
| torus/slice | | |
| subdivision/cube-2 | | |
| boole/mode-union | | |
| extrude/rectangle-caps-quadrangles | | |
| bend/strength | | |
| twist/angle | | |
| transform/ (any one case with all three rotations) | | |

## 3. Notes
Anything that behaved differently from R20's observation files (defaults, labels, ranges
you happened to see): TODO
