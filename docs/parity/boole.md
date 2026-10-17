# Boolean generator — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Unblocks: booleans. Folder: `tests/golden/boole/<case>/`.

Setup for every case: object A = default cube at the origin; object B = default cube
at position (50, 50, 50) (record units). Both under the boolean generator, A first.
Export the generator's result.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Notes |
|---|---|---|---|---|
| TODO (mode) | `mode` | enum | | list every option |
| TODO (quality option) | `high_quality` | bool | | |
| TODO (hide new edges) | `hide_new_edges` | bool | | |
| TODO (single object / other) | | | | |

Mode options (label → our key, e.g. `a_minus_b`, `union`, `intersect`, `a_without_b`): TODO

## Cases

| Case | Settings | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| mode-<key> | one case per mode, other options default | | 4 expected |
| quality-off | default mode, quality option flipped | | |
| hide-edges | default mode, hide-new-edges flipped | | |

## Behaviour notes
- What happens when the objects don't overlap / touch exactly: TODO
