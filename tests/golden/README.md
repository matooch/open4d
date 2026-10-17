# Golden reference data

Black-box exports from the reference app, one case per directory:

```
tests/golden/<feature>/<case>/case.toml   # required manifest
tests/golden/<feature>/<case>/mesh.obj    # optional exported geometry
```

Every case cites its observation note in `docs/parity/<feature>.md`. The manifest schema
and validation rules are in `docs/specs/INFRA.md` §3; `cargo test -p forge-parity-harness`
validates every case here. How to produce the data: `docs/PARITY.md`.

Only exported, documented formats go here (OBJ now; FBX/Alembic/images later). Never
native scene files or anything from the app's install folders (`docs/CLEANROOM.md`).
