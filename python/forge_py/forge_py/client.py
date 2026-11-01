"""The RPC client (docs/specs/RPC.md section 9). Standard library only."""

import json
import socket
import threading

from . import config
from .errors import ConnectionFailed, ForgeError, RpcError, Unauthorized

UNAUTHORIZED = -32001
_LOOPBACK = ("127.0.0.1", "localhost", "::1")


class Mesh(object):
    """A mesh in world space as returned by ``eval.mesh``."""

    def __init__(self, node, frame, points, polys):
        self.node = node
        self.frame = frame
        self.points = points
        self.polys = polys

    def bbox(self):
        """``(min, max)`` as ``[x, y, z]`` lists, or ``None`` for no points."""
        if not self.points:
            return None
        lo = [min(p[i] for p in self.points) for i in range(3)]
        hi = [max(p[i] for p in self.points) for i in range(3)]
        return lo, hi

    def __repr__(self):
        return "Mesh(node=%r, frame=%r, points=%d, polys=%d)" % (
            self.node, self.frame, len(self.points), len(self.polys))


def method_name(command_id):
    """``object.create`` -> ``object_create``."""
    return command_id.replace(".", "_")


def check_args(command, args):
    """Raises TypeError if ``args`` does not fit the command's ``args_schema``."""
    schema = command.get("args_schema") or {}
    props = schema.get("properties") or {}
    if schema.get("additionalProperties") is False or props:
        unknown = sorted(k for k in args if k not in props)
        if unknown and schema.get("additionalProperties", True) is False:
            raise TypeError("%s() got unexpected argument(s) %s; accepted: %s" % (
                method_name(command["id"]), ", ".join(unknown), ", ".join(sorted(props)) or "none"))
    missing = [k for k in schema.get("required") or [] if k not in args]
    if missing:
        raise TypeError("%s() missing required argument(s) %s" % (
            method_name(command["id"]), ", ".join(missing)))


class Client(object):
    """A connection to the editor (or ``forge-rpc-serve``).

    >>> forge = Client(port, token)
    >>> forge.object_create(type="forge.cube")
    {'node': 1}
    """

    def __init__(self, port=None, token=None, host="127.0.0.1", timeout=60.0):
        if host not in _LOOPBACK:
            raise ForgeError("only loopback hosts are allowed, not %r" % (host,))
        if port is None or token is None:
            cport, ctoken = config.resolve()
            port = cport if port is None else port
            token = ctoken if token is None else token
        self.port = port
        self._token = token
        self._lock = threading.Lock()
        self._next_id = 1
        self._commands = None
        self._by_method = None
        self._sock = None
        self._buf = b""
        try:
            self._sock = socket.create_connection((host, port), timeout=timeout)
        except OSError as e:
            raise ConnectionFailed("cannot connect to the editor on port %s: %s" % (port, e))

    # -- plumbing ---------------------------------------------------------------------

    def close(self):
        sock, self._sock = self._sock, None
        if sock is not None:
            try:
                sock.close()
            except OSError:
                pass

    def __enter__(self):
        return self

    def __exit__(self, *exc):
        self.close()
        return False

    def __repr__(self):
        return "Client(port=%r)" % (self.port,)

    def _read_line(self):
        while b"\n" not in self._buf:
            try:
                chunk = self._sock.recv(65536)
            except OSError as e:
                raise ConnectionFailed("connection error: %s" % (e,))
            if not chunk:
                raise ConnectionFailed("the server closed the connection")
            self._buf += chunk
        line, self._buf = self._buf.split(b"\n", 1)
        return line

    def call(self, method, params=None):
        """Sends one request and returns its ``result``."""
        with self._lock:
            if self._sock is None:
                raise ConnectionFailed("the client is closed")
            rid = self._next_id
            self._next_id += 1
            req = {"jsonrpc": "2.0", "id": rid, "method": method, "params": params or {}}
            if self._token:
                req["token"] = self._token
            data = json.dumps(req, separators=(",", ":"), ensure_ascii=True).encode("utf-8") + b"\n"
            try:
                self._sock.sendall(data)
            except OSError as e:
                raise ConnectionFailed("connection error: %s" % (e,))
            line = self._read_line()
        try:
            resp = json.loads(line.decode("utf-8"))
        except ValueError:
            raise ConnectionFailed("the server sent a line that is not JSON")
        if not isinstance(resp, dict):
            raise ConnectionFailed("the server sent a response that is not an object")
        err = resp.get("error")
        if err is not None:
            if not isinstance(err, dict):
                raise ConnectionFailed("malformed error object in the response")
            code = err.get("code")
            cls = Unauthorized if code == UNAUTHORIZED else RpcError
            raise cls(code, err.get("message", ""), err.get("data"))
        if resp.get("id") != rid:
            raise ConnectionFailed("response id %r does not match request id %r" % (resp.get("id"), rid))
        if "result" not in resp:
            raise ConnectionFailed("the response has neither result nor error")
        return resp["result"]

    # -- queries ------------------------------------------------------------------------

    def version(self):
        return self.call("app.version")

    def commands(self):
        """The ``command.list`` entries (id, label, undoable, args_schema)."""
        return self.call("command.list")["commands"]

    def scene(self):
        return self.call("scene.get")

    def node(self, id):
        return self.call("node.get", {"id": id})

    def timeline(self):
        return self.call("timeline.get")

    def eval_mesh(self, node, frame=None):
        params = {"node": node}
        if frame is not None:
            params["frame"] = frame
        r = self.call("eval.mesh", params)
        return Mesh(r["node"], r["frame"], r["points"], r["polys"])

    # -- commands -----------------------------------------------------------------------

    def run(self, id, args=None, **kwargs):
        """Runs command ``id`` (``command.run``) and returns the command's result."""
        merged = dict(args or {})
        merged.update(kwargs)
        return self.call("command.run", {"id": id, "args": merged})["result"]

    def _load_commands(self):
        if self._by_method is None:
            cmds = self.commands()
            explicit = set(dir(type(self)))
            by_method = {}
            for c in cmds:
                name = method_name(c["id"])
                if name not in explicit and name not in by_method:
                    by_method[name] = c
            self._commands = cmds
            self._by_method = by_method
        return self._by_method

    def command_names(self):
        """Names of the generated methods, sorted."""
        return sorted(self._load_commands())

    def __dir__(self):
        names = set(dir(type(self))) | set(self.__dict__)
        if self._sock is not None:
            names |= set(self._load_commands())
        return sorted(names)

    def __getattr__(self, name):
        if name.startswith("_"):
            raise AttributeError(name)
        cmd = self._load_commands().get(name)
        if cmd is None:
            raise AttributeError("%r object has no attribute or command %r" % (type(self).__name__, name))

        def method(*positional, **args):
            if positional:
                raise TypeError("%s() takes keyword arguments only" % (name,))
            check_args(cmd, args)
            return self.run(cmd["id"], args)

        method.__name__ = name
        props = sorted(((cmd.get("args_schema") or {}).get("properties") or {}))
        req = (cmd.get("args_schema") or {}).get("required") or []
        method.__doc__ = "%s (command %s, %s).\n\nKeyword arguments: %s" % (
            cmd.get("label", ""), cmd["id"],
            "undoable" if cmd.get("undoable") else "not undoable",
            ", ".join(p + ("" if p in req else "?") for p in props) or "none")
        return method
