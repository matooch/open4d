"""Exceptions raised by forge_py (docs/specs/RPC.md section 9.1)."""


class ForgeError(Exception):
    """Base class of every error forge_py raises on purpose."""


class ConnectionFailed(ForgeError):
    """Cannot connect, the socket closed, or the server sent something unusable."""


class RpcError(ForgeError):
    """The server answered with a JSON-RPC error object."""

    def __init__(self, code, message, data=None):
        super().__init__("[%s] %s" % (code, message))
        self.code = code
        self.message = message
        self.data = data


class Unauthorized(RpcError):
    """Token missing or wrong (-32001); the server closes the connection."""
