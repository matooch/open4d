# Cube primitive — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md). Folder: `tests/golden/cube/<case>/`.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| TODO (size, per axis?) | `size_x` / `size_y` / `size_z` | float | | | | | |
| TODO (segments per axis?) | `segments_x` / `_y` / `_z` | int | | | | | |
| TODO (fillet on/off?) | `fillet` | bool | | | | | |
| TODO (fillet radius) | `fillet_radius` | float | | | | | |
| TODO (fillet subdivision) | `fillet_segments` | int | | | | | |
| TODO (other options) | | | | | | | |

## Cases

| Case | Set | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | | |
| sizes | size = (100, 200, 300) | | |
| segments | segments = (2, 3, 4) | | vertex order matters: export as-is |
| fillet | fillet on, radius = 20, fillet segments = 3 | | |
| fillet-segments | fillet on + segments = (2, 2, 2) | | |

## Behaviour notes
- What happens at segments = 1 vs the minimum allowed: TODO
- Does fillet radius clamp relative to size? TODO
