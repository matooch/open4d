"""Starts a real ``forge-rpc-serve`` for the tests (docs/specs/RPC.md section 9.3)."""

import os
import secrets
import subprocess

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.abspath(os.path.join(HERE, "..", "..", ".."))


def find_binary():
    env = os.environ.get("FORGE_RPC_SERVE")
    if env:
        return env
    exe = "forge-rpc-serve" + (".exe" if os.name == "nt" else "")
    target = os.environ.get("CARGO_TARGET_DIR") or os.path.join(ROOT, "target")
    cands = [os.path.join(target, p, exe) for p in ("debug", "release")]
    for c in cands:
        if os.path.isfile(c):
            return c
    subprocess.check_call(["cargo", "build", "-q", "-p", "forge-rpc", "--bin", "forge-rpc-serve"], cwd=ROOT)
    for c in cands:
        if os.path.isfile(c):
            return c
    raise RuntimeError("forge-rpc-serve not found after building")


class ServerProcess(object):
    def __init__(self, token=None, extra_args=()):
        self.token = token if token is not None else secrets.token_hex(16)
        env = dict(os.environ)
        env["FORGE_RPC_TOKEN"] = self.token
        env.pop("FORGE_RPC_PORT", None)
        self.proc = subprocess.Popen(
            [find_binary()] + list(extra_args), stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            env=env, universal_newlines=True)
        line = self.proc.stdout.readline().strip()
        if not line.startswith("FORGE_RPC_PORT="):
            self.stop()
            raise RuntimeError("server did not report a port: %r" % (line,))
        self.port = int(line.split("=", 1)[1])

    def stop(self):
        try:
            self.proc.stdin.close()  # EOF makes the server exit
        except OSError:
            pass
        try:
            self.proc.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.proc.kill()
            self.proc.wait()
        self.proc.stdout.close()
