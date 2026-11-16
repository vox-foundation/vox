"""Framed stdio worker. The first action moves RPC off stdout.

Frame: uint32 header length, JSON header, uint64 body length, opaque body.
`debug.print`, `debug.sleep`, and `debug.crash` exist only when
`VOX_BRIDGE_DEBUG_OPS=1`.
"""

import json
import os
import platform
import struct
import sys
import time


def main() -> None:
    rpc_out = os.fdopen(os.dup(1), "wb", buffering=0)
    os.dup2(2, 1)
    sys.stdout = sys.stderr
    debug_ops = os.environ.get("VOX_BRIDGE_DEBUG_OPS") == "1"
    stdin = sys.stdin.buffer
    while True:
        frame = read_frame(stdin)
        if frame is None:
            return
        header, body = frame
        try:
            request = json.loads(header)
        except json.JSONDecodeError as exc:
            sys.stderr.write(f"vox_bridge: bad header: {exc}\n")
            return
        request_id = request.get("id")
        method = request.get("method", "")
        params = request.get("params") or {}
        try:
            if method == "shutdown":
                write_frame(rpc_out, result_header(request_id, {"ok": True}))
                return
            outcome = dispatch(method, params, body, debug_ops)
        except Exception as exc:
            message = str(exc)
            if not message.startswith("py-plan:") and not message.startswith("py:"):
                message = f"py: {message}"
            write_frame(rpc_out, error_header(request_id, message))
            continue
        if isinstance(outcome, tuple):
            value, reply_body = outcome
        else:
            value, reply_body = outcome, b""
        write_frame(rpc_out, result_header(request_id, value), reply_body)


def _pkg_version(dist_name: str):
    try:
        from importlib import metadata

        return metadata.version(dist_name)
    except Exception:
        return None


def _apply_seed_if_present(params: dict) -> None:
    seed = params.get("seed") if isinstance(params, dict) else None
    if isinstance(seed, int) and not isinstance(seed, bool):
        import random

        random.seed(seed)


def dispatch(method: str, params: dict, body: bytes, debug_ops: bool):
    if method == "execute_plan":
        _apply_seed_if_present(params)
        family = params.get("family") if isinstance(params, dict) else None
        # Imported here so numpy/polars stay unloaded until a plan runs, after
        # the stdout dup in main. Frame plans keep array_ops off the import path.
        if family == "frame":
            from vox_bridge.frame_ops import execute_plan

            return execute_plan(params, body)
        if family == "plot":
            from vox_bridge.plot_ops import execute_plot_plan

            return execute_plot_plan(params, body)
        if family in ("tensor_load", "tensor_run"):
            from vox_bridge.tensor_ops import execute_tensor_plan

            return execute_tensor_plan(params, body)
        if family == "escape_call":
            return execute_escape_call(params)
        from vox_bridge.array_ops import execute_plan

        return execute_plan(params, body)
    if method == "hello":
        return {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "pid": os.getpid(),
            "packages": {
                "python": platform.python_version(),
                "numpy": _pkg_version("numpy"),
                "polars": _pkg_version("polars"),
                "torch": _pkg_version("torch"),
            },
        }
    if method == "release":
        return {"released": list(params.get("ids") or [])}
    if method.startswith("debug."):
        if not debug_ops:
            raise RuntimeError("debug ops disabled")
        return debug_op(method, params)
    raise RuntimeError(f"unknown method {method}")


_DENIED_ESCAPE_MODULES = {
    "pickle",
    "marshal",
    "shelve",
    "_pickle",
    "subprocess",
    "_posixsubprocess",
    "importlib",
    "_imp",
    "imp",
}


def _is_denied_escape_target(module: str, symbol: str) -> bool:
    top = module.split(".")[0]
    if top in _DENIED_ESCAPE_MODULES:
        return True
    full = f"{module}.{symbol}" if symbol else module
    if full in ("eval", "exec", "builtins.eval", "builtins.exec"):
        return True
    if symbol == "__import__":
        return True
    if symbol == "load" and (module == "torch" or module.startswith("torch.")):
        return True
    if module in ("os", "posix", "nt") and (
        symbol in ("system", "popen")
        or symbol.startswith("spawn")
        or symbol.startswith("exec")
    ):
        return True
    return False


def _to_json_safe(val):
    if val is None or isinstance(val, (bool, int, float, str)):
        return val
    if hasattr(val, "tolist"):
        return _to_json_safe(val.tolist())
    if isinstance(val, (list, tuple)):
        return [_to_json_safe(x) for x in val]
    if isinstance(val, dict):
        return {str(k): _to_json_safe(v) for k, v in val.items()}
    return repr(val)


def execute_escape_call(params: dict):
    import importlib

    module = str(params.get("module", "")).strip()
    symbol = str(params.get("symbol", "")).strip()
    args = params.get("args") or []
    if not module or not symbol:
        raise RuntimeError("escape_call requires module and symbol")
    if _is_denied_escape_target(module, symbol):
        raise RuntimeError(f"py: denied Tier 2 Python target {module}.{symbol}")
    mod = importlib.import_module(module)
    fn = getattr(mod, symbol, None)
    if fn is None or not callable(fn):
        raise RuntimeError(f"py: module {module!r} has no callable {symbol!r}")
    out = fn(*args)
    return _to_json_safe(out)


def debug_op(method: str, params: dict):
    if method == "debug.print":
        print(params.get("text", ""), flush=True)
        return {"ok": True}
    if method == "debug.sleep":
        time.sleep(float(params.get("secs", 0)))
        return {"ok": True}
    if method == "debug.crash":
        os.abort()
    raise RuntimeError(f"unknown method {method}")


def read_exact(stream, size: int):
    buf = bytearray()
    while len(buf) < size:
        chunk = stream.read(size - len(buf))
        if not chunk:
            return None
        buf.extend(chunk)
    return bytes(buf)


def read_frame(stream):
    raw_len = read_exact(stream, 4)
    if raw_len is None:
        return None
    header_len = struct.unpack("<I", raw_len)[0]
    header = read_exact(stream, header_len) if header_len else b""
    if header is None:
        return None
    raw_body_len = read_exact(stream, 8)
    if raw_body_len is None:
        return None
    body_len = struct.unpack("<Q", raw_body_len)[0]
    body = b""
    if body_len:
        body = read_exact(stream, body_len)
        if body is None:
            return None
    if os.environ.get("VOX_BRIDGE_FRAME_DEBUG") == "1":
        sys.stderr.write(
            f"vox_bridge frame header_len={header_len} body_len={body_len} payload={header!r}\n"
        )
    return header, body


def write_frame(stream, header: bytes, body: bytes = b"") -> None:
    stream.write(struct.pack("<I", len(header)))
    stream.write(header)
    stream.write(struct.pack("<Q", len(body)))
    if body:
        stream.write(body)
    stream.flush()


def result_header(request_id, value) -> bytes:
    return json.dumps(
        {"id": request_id, "payload": {"type": "result", "value": value}}
    ).encode()


def error_header(request_id, message: str) -> bytes:
    return json.dumps(
        {
            "id": request_id,
            "payload": {"type": "error", "message": message, "code": 1},
        }
    ).encode()


if __name__ == "__main__":
    main()
