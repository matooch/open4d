# CLAUDE.md — Project rules (read first, every session)

Goal: a native Linux, Rust, functional-parity reimplementation of a classic DCC app
(working codename: **Forge3D**), including full MoGraph, all primitives/generators/modifiers.

## Hard rules (see docs/CLEANROOM.md)
1. NEVER disassemble, decompile, debug, or read binaries/resource/icon/preset/string files of the installed reference app.
2. NEVER copy assets, icons, layouts files, presets, shaders, or SDK headers. Do not use Maxon names/logos in code, UI, or crate names.
3. Reference app may be used ONLY as a black box via docs/PARITY.md procedures (run it, export, render, screenshot, record numbers).
4. Every feature spec in docs/specs cites its source: public manual, public SDK docs, or a parity observation file in docs/parity/.
5. Write the spec/observation FIRST, then tests, then code.

## Conventions
- One crate per domain; `cargo clippy -- -D warnings` and `cargo test` must pass before each commit.
- No `unsafe` without a `// SAFETY:` comment. Deterministic math (seeded RNG) so parity tests are reproducible.
- Everything is a plugin-registered node (objects, generators, deformers, effectors, fields, shaders, render engines).
- Parameters are data-driven descriptors (id, type, range, default, UI hint) so UI, undo, animation, and serialization derive from one definition.
- Work one checklist item at a time from docs/PLAN.md; tick it only when its parity test passes.

## Roles
- Planner / observer / implementer hand-off: docs/agents/README.md. An implementer (e.g. a local model) follows docs/agents/IMPLEMENTER.md and works one brief from docs/agents/tasks/ at a time.
