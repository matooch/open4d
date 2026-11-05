# Text spline (`forge.text_spline`; `crates/text`, `crates/generators/src/text_spline.rs`)

Sources: own design (brief docs/agents/tasks/039-text-spline.md) and the observation
[text.md](../parity/text.md) (O48), cases in `tests/golden/text` (each is an Extrude, Offset
20, caps off, of the Text spline; measured through the harness of
[splines-2.md](../parity/splines-2.md)). Public literature: the TrueType/OpenType outline
and metrics model (units per em, `hhea` ascender, descender and line gap, `hmtx` advances).
Values: Intermediate Points GENERATORS2.md §6.2; spline generators SPLINES.md.

## 0. What is and is not at parity

The meshes of O48 were all made with the reference app's default font. That font cannot be
changed by script, and we neither copy nor reverse-engineer it (CLEANROOM.md). So our
glyph outlines (a bundled, freely licensed font, §3) are **not** the reference's, and the
mesh of a case can never match vertex for vertex. Parity is therefore stated on rules that
do not depend on the font, each checked against the golden meshes
(`crates/generators/tests/text_parity.rs`):

| Rule | Cases | Reference measurement |
|---|---|---|
| Height is the em size: every coordinate scales by `height / 200` | `a`, `height` | bbox of `A` at 50 is exactly 1/4 of the bbox at 200, x and y |
| Left: the pen starts at x = 0 and the baseline is y = 0 | `a`, `two-lines` | `A` has min y 0; its first glyph starts at its left bearing |
| Middle: the **ink bounding box** of the line is centred on x = 0 | `align-center` | bbox x `-127.93 .. 127.93`; the shift equals half the sum of the line's ink min and max |
| Horizontal Spacing is added once per gap between glyphs | `spacing`, `align-center` | the second glyph moves by the spacing (19.92 for 20, i.e. 204 whole font units, within one font unit) |
| A newline starts a line one line height lower, at x = 0 | `two-lines` | line 1 is `a` unchanged; line 2 is 245.996 lower at height 200 (1.23 em) |
| Loops per glyph | `a`, `o`, `word`, `defaults`, `align-center`, `spacing`, `two-lines` | `A` 2, `O` 2, `T` 1, `e` 2, `x` 1, `t` 1, `F` 1, `o` 2, `r` 1, `g` 2, `B` 3 |
| Outer contours run clockwise, holes counter-clockwise (font order) | `a`, `o` | first loop negative signed area, second positive |
| Extrude grows toward -Z | all | every vertex has z 0 or -offset |
| Uniform Intermediate Points: `pieces * (number + 1)` points per contour | `intermediate` | `S`: 28 pieces, Number 4, 140 points per ring |
| Straight edges get no interior points (Adaptive) | `a` | `A` has 8 and 3 points, the corners |

Everything about the shape of a glyph is **parity-pending**: the lone observation is of one
unreplayable font. The observer should re-observe cases `a`, `o` and `word` with a freely
licensed font chosen in the GUI font picker (O48 Notes), for example DejaVu Sans; then the
default of §3 can be compared vertex for vertex.

## 1. Params (data-driven descriptors, `text_spline::node_type`)

| id | kind | default | meaning |
|---|---|---|---|
| `text` | string | `Text` | the text; `\n` starts a new line (`two-lines`) |
| `font` | string | `default` | `default` is the bundled font (§3); anything else is a family name looked up in the system font folders (§3); a missing family falls back to the bundled font |
| `align` | enum `left`, `middle`, `right` | `left` | labels Left, Middle, Right |
| `height` | float, distance | 200 | em size; range ±1e20 |
| `horizontal_spacing` | float, distance | 0 | added after every glyph but the last of a line |
| `vertical_spacing` | float, distance | 0 | added to the line step |
| `separate_letters` | bool | off | **Unsupported** (§5) |
| `plane` | enum `xy`, `zy`, `xz` | `xy` | as every spline (§6.1 of GENERATORS2.md) |
| `reverse` | bool | off | reverses the direction of every contour |
| `intermediate`, `intermediate_number`, `intermediate_angle`, `intermediate_max_length` | as every spline | Adaptive, 8, 5 deg, 5 | Intermediate Points (GENERATORS2.md §6.2), the shared `spline_sampling::add_params` |

Editing the text is changing the `text` parameter (undoable like any other); there is no
separate command. `forge.text_spline` is `Category::Generator`, `is_spline` is true, it has
no mesh of its own and is the input of Extrude, Lathe, Loft and Sweep like every spline.
Make Editable turns it into a `forge.spline` (SPLINES.md §3), keeping every loop.

## 2. Layout (`forge_text::layout`)

Let `u = height / units_per_em`. Per line, the pen starts at x = 0:
1. each character is mapped through the font's `cmap` (missing characters use glyph 0,
   `.notdef`); its outline is placed at `pen` and scaled by `u`; then
   `pen += advance * u + horizontal_spacing` (`spacing`). No kerning (`align-center`: the
   pair `AB` has none in the reference font; kerning is parity-pending).
2. Line `i` (0-based) has its baseline at `y = -i * (line_height * u + vertical_spacing)`
   with `line_height = ascender - descender + line_gap` from the font's `hhea` table
   (`two-lines`: the reference step is 2519 units of 2048 = 1.2300 em; that constant is the
   reference font's and is not reproduced, ours is the bundled font's 2384 units = 1.1641 em;
   adding `vertical_spacing` is **parity-pending**, the case leaves it at 0).
3. Alignment shifts a whole line in x by `-ref`, where `ref` is the **ink** bounding box of
   that line's outlines: `ref = (min + max) / 2` for Middle (`align-center`:
   shift `-1340` units = `-(30 + 2650) / 2`, so the bbox is exactly symmetric), `ref = max`
   for Right (**parity-pending**: only Left and Middle were observed), `0` for Left. A line
   without outline (spaces only) is not shifted. Alignment is per line (**parity-pending**:
   the observed two-line text is Left).

The planner's first reading was "relative to the line's advance"; the data say otherwise
(advances alone would give a shift of 1366 units, the ink centre gives the observed 1340).

Glyph outlines are emitted glyph by glyph in text order, line by line. Inside a glyph the
outer contours (winding like the glyph's largest contour) come before the holes, each group
in the font's own order: the reference lists a letter's outline before its counters
(`a`, `o`, `word`), while the bundled font stores the counter first. Outer contours keep
the font's winding, clockwise for TrueType, which is what makes the Auto Extrude direction
come out -Z (`defaults`, all cases). A negative `height` mirrors the outline through
the origin (**parity-pending**). A zero `height` or a text without any outline is
`BuildError::Failed`.

## 3. Fonts

Outlines come from `skrifa` (pure Rust, MIT/Apache-2.0, `cargo deny` clean, already in the
dependency tree; `ttf-parser` was tried first and fails `cargo deny` as unmaintained,
RUSTSEC-2026-0192). Quadratic pieces are raised to cubic exactly
(`c1 = p0 + 2/3 (q - p0)`, `c2 = p2 + 2/3 (q - p2)`), lines become straight pieces
(`splines::line`). A contour is closed; if its last point differs from its first a closing
line is added. Each contour is one closed `Curve`; the first is the spline and the rest are
its `extra` loops, exactly like the Ring and the Cogwheel (SPLINES.md §1.2), so the shared
sampler gives `pieces * (number + 1)` points to a Uniform contour (`intermediate`: 28 pieces,
Number 4, 140 points).

- **Default font**: DejaVu Sans, subset to U+0020-007E, U+00A0-00FF, U+2018-201D and U+20AC
  (hinting and layout tables dropped), `crates/text/fonts/DejaVuSans-subset.ttf`, 26 832
  bytes, 2048 units per em. The family name is unchanged and contains neither "Bitstream"
  nor "Vera", as the licence requires of a modified font; the licence text and copyright
  are in `crates/text/fonts/LICENSE-DejaVu.txt`. It is compiled in (`include_bytes!`), so
  the result never depends on the machine.
- **System fonts**: `font` other than `default` is matched case-insensitively against the
  family (name ids 16, then 1) of the `.ttf`, `.otf` and `.ttc` files whose file name
  contains the family name (spaces, `-` and `_` ignored; this avoids reading every font on
  every build), in sorted path order (so the choice is deterministic on one machine), in `/usr/share/fonts`,
  `/usr/local/share/fonts`, `~/.local/share/fonts`, `~/.fonts`, `/Library/Fonts`,
  `/System/Library/Fonts`, `~/Library/Fonts` and `C:\Windows\Fonts`. The first regular
  (non-bold, non-italic) face of the family wins. A family that is not found falls back to
  the bundled font; the golden tests and CI use the bundled font only.

## 4. Intermediate Points

The shared sampler (GENERATORS2.md §6.2) is applied to each contour. Default Adaptive, as
the reference (`defaults`). An all-straight contour (the letter `A`) gets only its corners
(`a`).

## 5. Unsupported

- `separate_letters` on: the reference makes one spline object per letter; one node here
  makes one spline. `BuildError::Unsupported("separate letters")`. Not observed.
- Variable-font axes, colour glyphs, bidirectional or complex shaping: characters map one
  to one through `cmap`.

## 6. Tests

`crates/text/tests/layout.rs` (outline and layout unit tests on the bundled font),
`crates/generators/tests/text_parity.rs` (the invariants of §0 against the nine golden
cases, with the Extrude pipeline) and the registry/Make Editable checks in
`crates/generators/tests/text_spline.rs`. Glyph shapes are not compared.
