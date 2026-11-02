"""The RPC client (docs/specs/RPC.md section 9). Standard library only."""

import base64
import collections
import json
import socket
import threading
import time

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


Notification = collections.namedtuple("Notification", ["method", "params"])
Notification.__doc__ = "A server notification such as ``frame.changed`` (docs/specs/RPC.md section 10)."


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
        self._timeout = timeout
        self._buf = b""
        self._notes = collections.deque()
        self._callback = None
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

    def _read_line(self, deadline=None):
        """The next line; ``None`` if ``deadline`` (a monotonic time) passes first."""
        while b"\n" not in self._buf:
            if deadline is not None:
                left = deadline - time.monotonic()
                if left <= 0:
                    return None
                self._sock.settimeout(left)
            try:
                chunk = self._sock.recv(65536)
            except socket.timeout:
                if deadline is not None:
                    return None
                raise ConnectionFailed("connection error: timed out")
            except OSError as e:
                raise ConnectionFailed("connection error: %s" % (e,))
            if not chunk:
                raise ConnectionFailed("the server closed the connection")
            self._buf += chunk
        line, self._buf = self._buf.split(b"\n", 1)
        return line

    @staticmethod
    def _parse(line):
        try:
            msg = json.loads(line.decode("utf-8"))
        except ValueError:
            raise ConnectionFailed("the server sent a line that is not JSON")
        if not isinstance(msg, dict):
            raise ConnectionFailed("the server sent a response that is not an object")
        return msg

    def _note(self, msg):
        """Queues a notification and tells the callback; returns the notification."""
        note = Notification(msg["method"], msg.get("params") or {})
        self._notes.append(note)
        callback = self._callback
        if callback is not None:
            callback(note.method, note.params)
        return note

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
                self._sock.settimeout(self._timeout)
                self._sock.sendall(data)
            except OSError as e:
                raise ConnectionFailed("connection error: %s" % (e,))
            while True:
                if self._sock is None:
                    raise ConnectionFailed("the client is closed")
                self._sock.settimeout(self._timeout)
                resp = self._parse(self._read_line())
                if "id" not in resp and isinstance(resp.get("method"), str):
                    self._note(resp)
                    continue
                break
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

    # -- notifications, screenshots, jobs ---------------------------------------------

    def subscribe(self, *kinds):
        """Receive notifications of ``kinds`` (``scene``, ``frame``, ``selection``,
        ``commands``; none given means all). Returns the subscription."""
        return self.call("events.subscribe", {"kinds": list(kinds)} if kinds else {})["kinds"]

    def unsubscribe(self, *kinds):
        return self.call("events.unsubscribe", {"kinds": list(kinds)} if kinds else {})["kinds"]

    def on_notification(self, callback):
        """Calls ``callback(method, params)`` for every notification read from now on
        (on the thread that reads it); ``None`` removes it. Notifications are also queued
        for ``next_notification``."""
        self._callback = callback

    def next_notification(self, timeout=None):
        """The next queued or arriving :class:`Notification`; ``None`` after ``timeout``
        seconds (``None`` waits forever). Notifications arrive only while a call or this
        method reads the socket."""
        deadline = None if timeout is None else time.monotonic() + timeout
        with self._lock:
            while True:
                if self._notes:
                    return self._notes.popleft()
                if self._sock is None:
                    raise ConnectionFailed("the client is closed")
                if deadline is None:
                    self._sock.settimeout(None)
                line = self._read_line(deadline)
                if line is None:
                    return None
                msg = self._parse(line)
                if "id" in msg or not isinstance(msg.get("method"), str):
                    raise ConnectionFailed("unexpected response while waiting for a notification")
                self._note(msg)

    def screenshot(self, width=None, height=None):
        """A PNG of the editor's viewport as ``bytes`` (``RpcError`` -32000 without a viewport)."""
        params = {}
        if width is not None:
            params["width"] = width
        if height is not None:
            params["height"] = height
        return base64.b64decode(self.call("viewport.screenshot", params)["data"])

    def job_start(self, kind, **args):
        """Starts a long job; returns its id. Progress arrives as ``job.progress``
        notifications, the end as ``job.finished``."""
        return self.call("job.start", {"kind": kind, "args": args})["job_id"]

    def job_cancel(self, id):
        return self.call("job.cancel", {"id": id})

    def jobs(self):
        return self.call("job.list")["jobs"]

    def job_wait(self, id, timeout=None, on_progress=None):
        """Waits for job ``id`` to end; returns the ``job.finished`` params
        (``state`` is ``done``, ``cancelled`` or ``failed``). ``on_progress(fraction,
        message)`` is called for its progress. Raises ``ForgeError`` on timeout."""
        deadline = None if timeout is None else time.monotonic() + timeout
        while True:
            left = None if deadline is None else max(0.0, deadline - time.monotonic())
            note = self.next_notification(left)
            if note is None:
                raise ForgeError("job %s did not finish in time" % (id,))
            if note.params.get("id") != id:
                continue
            if note.method == "job.progress" and on_progress is not None:
                on_progress(note.params["fraction"], note.params["message"])
            elif note.method == "job.finished":
                return note.params

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
