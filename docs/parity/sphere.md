# Sphere primitive — observation

Status: not observed · App version: TODO · OS: TODO · Observer: TODO · Date: TODO
Exporter settings: [export-settings.md](export-settings.md). Folder: `tests/golden/sphere/<case>/`.

## Parameters (as shown in the app)

| App label | Our key | Type | Default | Min | Max | Unit shown | Notes |
|---|---|---|---|---|---|---|---|
| TODO (radius) | `radius` | float | | | | | |
| TODO (segments) | `segments` | int | | | | | one number or two? |
| TODO (type) | `type` | enum | | | | | list every option |
| TODO (other) | | | | | | | |

Type options (label → our key, e.g. `standard`, `tetra`, …): TODO

## Cases

| Case | Set | `golden info` (vertices / faces / size) | Notes |
|---|---|---|---|
| defaults | – | | |
| radius | radius = 37.5 | | |
| segments-8 | segments = 8 | | |
| segments-odd | segments = 9 | | odd counts reveal pole/seam rules |
| type-<key> | one case per type option, else defaults | | |
| type-<key>-segments | each non-default type with segments = 8 | | does segments mean the same thing? |

## Behaviour notes
- Where the seam is (which side) and whether poles are triangles or n-gons: TODO
