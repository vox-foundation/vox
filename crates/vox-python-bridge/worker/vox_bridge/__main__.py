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
        header, _body = frame
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
            value = dispatch(method, params, debug_ops)
        except Exception as exc:
            write_frame(rpc_out, error_header(request_id, f"py: {exc}"))
            continue
        write_frame(rpc_out, result_header(request_id, value))


def dispatch(method: str, params: dict, debug_ops: bool):
    if method == "hello":
        return {
            "python": platform.python_version(),
            "platform": platform.platform(),
            "pid": os.getpid(),
        }
    if method == "release":
        return {"released": list(params.get("ids") or [])}
    if method.startswith("debug."):
        if not debug_ops:
            raise RuntimeError("debug ops disabled")
        return debug_op(method, params)
    raise RuntimeError(f"unknown method {method}")


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
