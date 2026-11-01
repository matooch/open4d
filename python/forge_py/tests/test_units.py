"""Tests that need no editor: name mapping, schema checks, settings, a fake server."""

import socket
import threading
import unittest

from forge_py import Client, ConnectionFailed, ForgeError, check_args, config, method_name

CREATE = {"id": "object.create", "label": "Create", "undoable": True, "args_schema": {
    "additionalProperties": False, "properties": {"type": {"type": "string"}}, "required": ["type"]}}


class Names(unittest.TestCase):
    def test_mapping(self):
        self.assertEqual(method_name("object.create"), "object_create")
        self.assertEqual(method_name("file.export_obj"), "file_export_obj")

    def test_check_args(self):
        check_args(CREATE, {"type": "forge.cube"})
        with self.assertRaisesRegex(TypeError, "missing required argument.*type"):
            check_args(CREATE, {})
        with self.assertRaisesRegex(TypeError, "unexpected argument.*bogus"):
            check_args(CREATE, {"type": "x", "bogus": 1})

    def test_loopback_only(self):
        with self.assertRaises(ForgeError):
            Client(1, "t", host="192.0.2.1")


class Settings(unittest.TestCase):
    def test_env_beats_file(self):
        f = {"rpc": {"port": 1111, "token": "file"}}
        self.assertEqual(config.resolve({"FORGE_RPC_PORT": "2222", "FORGE_RPC_TOKEN": "env"}, f), (2222, "env"))
        self.assertEqual(config.resolve({}, f), (1111, "file"))
        self.assertEqual(config.resolve({}, {}), (7323, None))

    def test_empty_token_and_bad_port(self):
        f = {"rpc": {"token": "file"}}
        self.assertEqual(config.resolve({"FORGE_RPC_TOKEN": "", "FORGE_RPC_PORT": "zzz"}, f), (7323, "file"))
        self.assertEqual(config.resolve({}, {"rpc": {"token": ""}}), (7323, None))

    def test_settings_path(self):
        self.assertEqual(config.settings_path({"FORGE_SETTINGS": "/a"}), "/a")
        self.assertTrue(config.settings_path({"XDG_CONFIG_HOME": "/x"}).replace("\\", "/").endswith("x/forge3d/settings.json"))
        self.assertIsNone(config.settings_path({}))


class FakeServer(object):
    """Answers each request with a canned line."""

    def __init__(self, reply):
        self.listener = socket.socket()
        self.listener.bind(("127.0.0.1", 0))
        self.listener.listen(1)
        self.port = self.listener.getsockname()[1]
        self.reply = reply
        self.thread = threading.Thread(target=self._serve, daemon=True)
        self.thread.start()

    def _serve(self):
        conn, _ = self.listener.accept()
        with conn:
            buf = b""
            while b"\n" not in buf:
                chunk = conn.recv(4096)
                if not chunk:
                    return
                buf += chunk
            conn.sendall(self.reply)

    def close(self):
        self.listener.close()


class BadServers(unittest.TestCase):
    def check(self, reply, pattern):
        s = FakeServer(reply)
        try:
            with Client(s.port, "t", timeout=5) as c:
                with self.assertRaisesRegex(ConnectionFailed, pattern):
                    c.version()
        finally:
            s.close()

    def test_mismatched_id(self):
        self.check(b'{"jsonrpc":"2.0","id":99,"result":{}}\n', "does not match")

    def test_not_json(self):
        self.check(b"hello\n", "not JSON")

    def test_closed_without_reply(self):
        self.check(b"", "closed")

    def test_no_result(self):
        self.check(b'{"jsonrpc":"2.0","id":1}\n', "neither result")


class NotificationsOnTheWire(unittest.TestCase):
    def test_notifications_before_a_response_are_queued_not_mistaken_for_it(self):
        reply = (b'{"jsonrpc":"2.0","method":"frame.changed","params":{"frame":3}}\n'
                 b'{"jsonrpc":"2.0","method":"scene.changed","params":{"revision":9}}\n'
                 b'{"jsonrpc":"2.0","id":1,"result":{"ok":true}}\n')
        s = FakeServer(reply)
        seen = []
        try:
            with Client(s.port, "t", timeout=5) as c:
                c.on_notification(lambda method, params: seen.append((method, params)))
                self.assertEqual(c.version(), {"ok": True})
                first = c.next_notification(timeout=1)
                self.assertEqual((first.method, first.params), ("frame.changed", {"frame": 3}))
                self.assertEqual(c.next_notification(timeout=1).method, "scene.changed")
        finally:
            s.close()
        self.assertEqual([m for m, _ in seen], ["frame.changed", "scene.changed"])


if __name__ == "__main__":
    unittest.main()
