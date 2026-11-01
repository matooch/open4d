"""Acceptance scenario S2 through forge-py (docs/roadmap/README.md, docs/manual/automation.md).

An agent connects to a running editor (here: forge-rpc-serve, the editor without a window),
creates a sphere, keys its position at frames 0 and 30, plays the timeline and reads back the
evaluated position at frame 15. There is no UI clock in the server, so "play" is checked as
the playing flag; the frame is then moved with timeline_set_frame.
"""

import unittest

from forge_py import Client

from .server import ServerProcess


def centre_x(mesh):
    lo, hi = mesh.bbox()
    return (lo[0] + hi[0]) / 2.0


class S2(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.server = ServerProcess()

    @classmethod
    def tearDownClass(cls):
        cls.server.stop()

    def test_key_a_sphere_play_and_read_frame_15(self):
        with Client(self.server.port, self.server.token, timeout=30) as forge:
            forge.file_new()
            sphere = forge.object_create(type="forge.sphere")["node"]
            forge.timeline_settings(fps=30, start=0, end=30)
            forge.key_set(node=sphere, track="transform.pos.x", frame=0, value=0.0)
            forge.key_set(node=sphere, track="transform.pos.x", frame=30, value=300.0)

            forge.timeline_play()
            self.assertTrue(forge.timeline()["playing"])
            forge.timeline_stop()
            self.assertFalse(forge.timeline()["playing"])

            forge.timeline_set_frame(frame=15)
            self.assertEqual(forge.timeline()["frame"], 15)
            at15 = centre_x(forge.eval_mesh(sphere))
            at0 = centre_x(forge.eval_mesh(sphere, frame=0))
            self.assertAlmostEqual(at15 - at0, 150.0, delta=0.01)
            self.assertAlmostEqual(centre_x(forge.eval_mesh(sphere, frame=30)) - at0, 300.0, delta=0.01)


if __name__ == "__main__":
    unittest.main()
