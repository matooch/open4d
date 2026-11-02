# forge-py

A small, pure-Python client for the Forge3D editor's JSON-RPC server
(`docs/specs/RPC.md`). Standard library only, Python 3.9 or newer. Nothing is embedded: the
editor (or `forge-rpc-serve`) runs as its own process and you talk to it over
`127.0.0.1`.

## Install

```sh
pip install ./python/forge_py        # or just put python/forge_py on PYTHONPATH
```

## Connect

The editor needs a token (`FORGE_RPC_TOKEN` or `rpc.token` in the settings file, see
RPC.md section 2). `Client()` reads port and token the same way the server does; or pass
them explicitly:

```python
from forge_py import Client

with Client(port=7323, token="secret") as forge:
    print(forge.version())
```

For scripts and tests there is a headless server (no window):

```sh
FORGE_RPC_TOKEN=secret cargo run -p forge-rpc --bin forge-rpc-serve
# prints FORGE_RPC_PORT=<port>
```

## Use

```python
from forge_py import Client, RpcError

forge = Client()

# Every command in the registry is a method: object.create -> object_create.
cube = forge.object_create(type="forge.cube")["node"]
forge.param_set(node=cube, param="size_y", value=60)     # angles in degrees
forge.timeline_set_frame(frame=12)

# Or by id, with a dict and/or keywords.
forge.run("transform.set", node=cube, pos=[0, 100, 0])

# Queries.
forge.version()            # {"name": "forge3d", "version": ..., "protocol": 1}
forge.scene()              # fps, frame, selected, roots (nested node objects)
forge.node(cube)           # one node: type, params in user units, tags, children
forge.timeline()
mesh = forge.eval_mesh(cube, frame=0)    # world space, deformers and animation applied
print(len(mesh.points), len(mesh.polys), mesh.bbox())

# Errors.
try:
    forge.object_create(type="no.such.type")
except RpcError as e:
    print(e.code, e.message)          # -32000 ...
```

Command methods take keyword arguments only; an unknown or missing argument is a
`TypeError` raised before anything is sent. `forge.command_names()` lists the generated
methods and `help(forge.object_create)` shows the arguments. `forge.call(method, params)`
sends any RPC method.

Errors: `ForgeError` (base), `ConnectionFailed`, `RpcError(code, message, data)` and its
subclass `Unauthorized` (wrong or missing token).

## Notifications, screenshots, jobs

```python
forge.subscribe("frame", "selection")      # also "scene", "commands"; none = all
forge.timeline_set_frame(frame=12)
note = forge.next_notification(timeout=5)   # Notification(method="frame.changed", params={"frame": 12})
forge.on_notification(lambda method, params: print(method, params))

open("view.png", "wb").write(forge.screenshot(800, 600))   # needs the editor window's GPU

job = forge.job_start("export.obj_sequence", dir="/tmp/seq", start=0, end=90)
done = forge.job_wait(job, on_progress=lambda fraction, message: print(fraction, message))
forge.job_cancel(job)                       # stops a running job
```

Notifications are read while a call or `next_notification` waits; there is no background
thread. `forge-rpc-serve` has no viewport, so `screenshot` is an `RpcError` there.

## Tests

```sh
cd python/forge_py && python3 -m unittest discover -t . -s tests
```

The tests start a real `forge-rpc-serve` (from `$FORGE_RPC_SERVE`, `target/`, or built with
cargo) on a free port with a random token.
