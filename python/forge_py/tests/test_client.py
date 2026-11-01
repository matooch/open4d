"""Tests against a real forge-rpc-serve (docs/specs/RPC.md section 9.3)."""

import socket
import unittest

from forge_py import Client, ConnectionFailed, RpcError, Unauthorized

from .server import ServerProcess


class LiveTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ServerProcess()

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()

    def setUp(self):
        self.forge = Client(self.server.port, self.server.token, timeout=30)
        self.addCleanup(self.forge.close)
        self.forge.file_new()

    def cube(self):
        return self.forge.object_create(type="forge.cube")["node"]


class Queries(LiveTest):
    def test_version(self):
        v = self.forge.version()
        self.assertEqual(v["name"], "forge3d")
        self.assertEqual(v["protocol"], 1)

    def test_commands_match_raw_list(self):
        raw = self.forge.call("command.list")["commands"]
        self.assertEqual(self.forge.commands(), raw)
        names = self.forge.command_names()
        self.assertIn("object_create", names)
        self.assertIn("timeline_set_frame", names)
        self.assertEqual(len(names), len(set(names)))

    def test_create_and_read_back(self):
        node = self.cube()
        scene = self.forge.scene()
        self.assertEqual([r["id"] for r in scene["roots"]], [node])
        self.assertEqual(scene["selected"], [node])
        got = self.forge.node(node)
        self.assertEqual((got["id"], got["type"]), (node, "forge.cube"))

    def test_param_set_visible(self):
        node = self.cube()
        self.forge.param_set(node=node, param="size_y", value=60)
        self.assertEqual(self.forge.node(node)["params"]["size_y"], 60)
        lo, hi = self.forge.eval_mesh(node).bbox()
        self.assertAlmostEqual(hi[1] - lo[1], 60, places=6)

    def test_eval_mesh(self):
        node = self.cube()
        m = self.forge.eval_mesh(node)
        self.assertEqual((len(m.points), len(m.polys)), (8, 6))
        lo, hi = m.bbox()
        self.assertTrue(all(h > l for l, h in zip(lo, hi)))
        self.assertEqual(self.forge.eval_mesh(node, frame=5).frame, 5)
        self.assertEqual(self.forge.timeline()["frame"], 0)  # the timeline did not move

    def test_timeline(self):
        self.forge.timeline_set_frame(frame=7)
        t = self.forge.timeline()
        self.assertEqual(t["frame"], 7)
        self.assertEqual(self.forge.scene()["frame"], 7)

    def test_run_with_dict_and_keywords(self):
        r = self.forge.run("object.create", {"type": "forge.cube"})
        self.assertIn("node", r)
        self.assertIn("node", self.forge.run("object.create", type="forge.sphere"))

    def test_generated_method_doc(self):
        self.assertIn("object.create", self.forge.object_create.__doc__)
        self.assertIn("object_create", dir(self.forge))


class Errors(LiveTest):
    def test_wrong_token(self):
        with Client(self.server.port, "nope", timeout=30) as bad:
            with self.assertRaises(Unauthorized) as cm:
                bad.version()
        self.assertEqual(cm.exception.code, -32001)

    def test_missing_token(self):
        with Client(self.server.port, None, timeout=30) as bad:
            if bad._token is not None:  # a token from the environment would defeat this test
                self.skipTest("a token is configured in the environment")
            with self.assertRaises(Unauthorized):
                bad.version()

    def test_unknown_node_and_usable_after(self):
        with self.assertRaises(RpcError) as cm:
            self.forge.node(99999)
        self.assertEqual(cm.exception.code, -32000)
        self.assertEqual(self.forge.version()["name"], "forge3d")

    def test_failing_command(self):
        with self.assertRaises(RpcError) as cm:
            self.forge.object_create(type="no.such.type")
        self.assertEqual(cm.exception.code, -32000)
        self.assertEqual(self.forge.scene()["roots"], [])

    def test_unknown_method(self):
        with self.assertRaises(RpcError) as cm:
            self.forge.call("no.such")
        self.assertEqual(cm.exception.code, -32601)

    def test_bad_params(self):
        with self.assertRaises(RpcError) as cm:
            self.forge.call("node.get", {"id": "x"})
        self.assertEqual(cm.exception.code, -32602)

    def test_type_errors_make_no_request(self):
        with self.assertRaisesRegex(TypeError, "missing required"):
            self.forge.object_create()
        with self.assertRaisesRegex(TypeError, "unexpected argument"):
            self.forge.object_create(type="forge.cube", bogus=1)
        with self.assertRaisesRegex(TypeError, "keyword"):
            self.forge.object_create("forge.cube")
        self.assertEqual(self.forge.scene()["roots"], [])

    def test_attribute_error(self):
        with self.assertRaises(AttributeError):
            self.forge.no_such_command


class Connection(unittest.TestCase):
    def test_refused(self):
        s = socket.socket()
        s.bind(("127.0.0.1", 0))
        port = s.getsockname()[1]
        s.close()
        with self.assertRaises(ConnectionFailed):
            Client(port, "t", timeout=5)

    def test_close_twice_and_context_manager(self):
        srv = ServerProcess()
        try:
            with Client(srv.port, srv.token) as c:
                self.assertEqual(c.version()["name"], "forge3d")
            c.close()
            with self.assertRaises(ConnectionFailed):
                c.version()
        finally:
            srv.stop()

    def test_environment_resolution(self):
        import os
        srv = ServerProcess()
        old = {k: os.environ.get(k) for k in ("FORGE_RPC_PORT", "FORGE_RPC_TOKEN")}
        os.environ["FORGE_RPC_PORT"] = str(srv.port)
        os.environ["FORGE_RPC_TOKEN"] = srv.token
        try:
            with Client() as c:
                self.assertEqual(c.version()["name"], "forge3d")
        finally:
            for k, v in old.items():
                if v is None:
                    os.environ.pop(k, None)
                else:
                    os.environ[k] = v
            srv.stop()


if __name__ == "__main__":
    unittest.main()
