# 130 · Image IO, texture cache, colour management

Status: done · Area: J · Depends on: – · Observations: – · Touches: image (new)

## Goal
Load PNG/JPEG/TIFF/HDR/EXR, cache textures with mips, linear/sRGB handling.

## Inputs
- Area design: [../../roadmap/areas/J-materials.md](../../roadmap/areas/J-materials.md)

## Design decisions (planner)
- `image` + `exr` crates; working space linear sRGB; OCIO out of scope for v1.

## Done when
- [x] Round-trip tests for each format.
- [x] The gate passes and BACKLOG.md is updated; Finish below is followed.

## Finish (every brief)
1. Follow [../IMPLEMENTER.md](../IMPLEMENTER.md): spec first, then tests, then code. Don't edit golden data or observation forms, and never loosen a tolerance.
2. `rustup update stable`, so you use the toolchain CI uses. Then `cargo fmt --all`, then `sh scripts/gate.sh` until it prints `GATE: all passed`.
3. Update `docs/parity/STATUS.md`, if this brief adds or changes parity results, and set this brief's row in [../../roadmap/BACKLOG.md](../../roadmap/BACKLOG.md) to `done`.
4. Commit with a summary line, what you did and the parity result, then a `Co-Authored-By:` trailer naming your model.
5. Fill in the Report below and set the Status line to `done`, or `blocked` with the reason.

## Report (implementer fills in)
- Result: spec `docs/specs/IMAGE.md`; new crate `forge-image` (`crates/image`, deps `image` 0.25 with png/jpeg/tiff/hdr and `exr` 1; licences MIT/Apache-2.0 and BSD-3-Clause, `cargo deny check` passes with no exception). 42 tests in `crates/image/tests/{colour,formats,exr_layers,mips,cache}.rs`; all images generated in tests. No parity cases.
- Rules (IMAGE.md): working space linear sRGB; sRGB transfer with odd extension and an exact 8-bit table; tag `Srgb|Linear|Raw` applies to integer files only (float files are always linear; `Raw` is untouched, no alpha conversion); alpha order is to straight, transfer, to requested mode; EXR alpha is premultiplied by convention, the others straight. `decode(bytes, &Limits)` and `decode_exr_layers` are pure, total and check size limits before allocating (fuzz entry points; truncation and byte-flip tests never panic). Formats and depths: PNG/TIFF 8 and 16 write+read (exact on codes), JPEG 8 (PSNR bound), HDR f32 (RGBE, below 1 % error), EXR f16/f32 read and write, multi-layer read (multipart plus legacy dotted channel names) and multipart write (`encode_exr_layers`, for AOVs in brief 139). Mips: area-weighted box filter on linear premultiplied RGBA, exact mean at every level for odd sizes. Cache: `TextureCache` keyed by resolved path + options with an (mtime, size) stamp per entry, LRU under a byte budget, oversize textures returned but not stored, stats (hits, misses, evictions, invalidations), one mutex with decode outside the lock, `get_ref(&str)` for `file_path` params (empty = none).
- Deviations: no `forge-platform` dependency (no derived data is written to disk, so no cache dir is needed); EXR is read and written through the `exr` crate directly, the `image` crate's `exr` feature is off; no float TIFF write; HDR and JPEG drop alpha; two threads loading the same missing key may both decode (one result kept).
- Files changed: crates/image/*, docs/specs/IMAGE.md, docs/roadmap/BACKLOG.md, this brief, Cargo.lock.
- Open questions: who builds the render-output AOV writer on top of `encode_exr_layers` (brief 139); whether a persistent on-disk mip cache is wanted.
- Gate: `GATE: all passed` (workspace tests all green, cargo deny ok, cleanroom ok, golden check ok: 471 cases).
