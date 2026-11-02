"""Port and token resolution, the same order as the server (docs/specs/RPC.md section 2)."""

import json
import os

DEFAULT_PORT = 7323


def settings_path(env=None):
    env = os.environ if env is None else env
    if env.get("FORGE_SETTINGS"):
        return env["FORGE_SETTINGS"]
    base = env.get("XDG_CONFIG_HOME")
    if not base:
        home = env.get("HOME")
        if not home:
            return None
        base = os.path.join(home, ".config")
    return os.path.join(base, "forge3d", "settings.json")


def _read_file(path):
    if not path:
        return {}
    try:
        with open(path, "r", encoding="utf-8") as f:
            data = json.load(f)
    except (OSError, ValueError):
        return {}
    return data if isinstance(data, dict) else {}


def resolve(env=None, file_data=None):
    """Returns (port, token or None). Environment beats the settings file."""
    env = os.environ if env is None else env
    if file_data is None:
        file_data = _read_file(settings_path(env))
    rpc = file_data.get("rpc")
    rpc = rpc if isinstance(rpc, dict) else {}
    port = None
    try:
        port = int(str(env.get("FORGE_RPC_PORT", "")).strip())
    except ValueError:
        pass
    if port is None or not 0 <= port <= 65535:
        fp = rpc.get("port")
        port = fp if isinstance(fp, int) and not isinstance(fp, bool) and 0 <= fp <= 65535 else DEFAULT_PORT
    token = env.get("FORGE_RPC_TOKEN") or None
    if token is None:
        ft = rpc.get("token")
        token = ft if isinstance(ft, str) and ft else None
    return port, token
