# Pose Morph — observation (O37)

Status: not observed (0 of 7 cases; setting the pose-morph strength track failed — the script aborts without an error inside the track code; details in ~/forge-observer/notes.md) · App version: Cinema 4D 2025.1.0 (25.010) · OS: macOS 26.7.2 (25G332, arm64) · Observer: Hermes · Date: 2026-11-20
Method: scripted. Unblocks: H/107. Level A.

## Setup

- **Mesh:** a sphere (radius 100, segments 16), Made Editable.
- **Tag:** add a **Pose Morph** tag with point morphing enabled. The tag's base pose is the
  sphere as made.
- **Morph A:** move every point with y > 50 by (0, 50, 0).
- **Morph B:** scale every point's x by 1.5.

Set the morph points through the script, in the tag's edit mode or its scripting
interface. Record the exact point displacement rules in `case.toml`.

Output: `tests/golden/pose-morph/<case>/mesh.obj`, with `feature = "pose-morph"`.

## Parameters (fill once)
| Label | Our key | Default | Options / Min / Max |
|---|---|---|---|

No parameter values were read: the tag never reached a state where its strength could be
keyed or read back.

## Cases

| Case | Strengths | Mode | Notes |
|---|---|---|---|
| base | A 0 %, B 0 % | relative | not observed: pose-morph strength track not settable (see Notes) |
| a-50 | A 50 % | relative | not observed: same reason |
| a-100 | A 100 % | relative | not observed: same reason |
| a-150 | A 150 % (if allowed) | relative | not observed: same reason |
| ab-mix | A 100 %, B 50 % | relative | not observed: same reason |
| absolute | A 100 %, B 50 % | absolute (or the other mode) | not observed: same reason |
| psr-morph | a second tag with position/rotation morphing: morph C rotates the object H 90° | 50 % | not observed: same reason |

## Notes

not observed: setting the pose-morph strength track failed (the script aborts silently in the
track code; details in ~/forge-observer/notes.md)

What did work in the probe: the sphere was built and made editable, the Pose Morph tag was
added with point morphing on, and both stored morphs were created, named and filled with the
point rules the Setup describes. No case was exported, because every case needs a strength
value and the strength could only be produced through a keyframe track.

Where it stopped: the strength is a two-level parameter of the tag. Building a track for that
parameter and recording a key on it aborts the whole script with no message at all — the
bridge only reports that the script returned nothing. Setting the tag's **Mode** to
**Animate** first, as the app's own workflow requires, did not help, and the Mode parameter
reads back empty even immediately after being set.

Tried (3 attempts, budget spent): (1) building the track from the parameter address read back
from the tag, (2) setting Mode to Animate before building the track, (3) isolating the track
insertion, the key fill and the value write into separate minimal scripts — the abort lands
in the track code before any of them can report. Not retried in later runs; see
~/forge-observer/notes.md ("Pose morph: where it stands") for the next steps if the owner wants
to pick it up.
