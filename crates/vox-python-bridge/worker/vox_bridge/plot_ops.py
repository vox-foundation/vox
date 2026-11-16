"""std.plot handlers for execute_plan (matplotlib Agg when present, stdlib PNG fallback).

Reads the CSV/frame inside the worker without sending rows back to the host.
"""

import os
import struct
import zlib

from vox_bridge.frame_ops import PlanError, collect


def _build_frame(ops: list):
    from vox_bridge.frame_ops import HANDLERS
    import polars as pl

    lf = None
    exprs = []
    for step in ops:
        if not isinstance(step, dict):
            raise PlanError("py-plan: plot frame op is not an object")
        name = step.get("name")
        handler = HANDLERS.get(name) if isinstance(name, str) else None
        if handler is None:
            raise PlanError(f"py-plan: unknown frame op {name!r}")
        lf = handler(lf, step, exprs)
    if lf is None:
        raise PlanError("py-plan: plot plan did not build a frame")
    if not isinstance(lf, pl.DataFrame):
        lf = collect(lf, {"name": "collect"}, exprs)
    return lf


def _encode_rgb_png(width: int, height: int, rgb: bytearray) -> bytes:
    def chunk(tag: bytes, data: bytes) -> bytes:
        body = tag + data
        return struct.pack(">I", len(data)) + body + struct.pack(">I", zlib.crc32(body) & 0xFFFFFFFF)

    stride = width * 3
    raw = bytearray()
    for y in range(height):
        raw.append(0)
        start = y * stride
        raw.extend(rgb[start : start + stride])
    ihdr = struct.pack(">IIBBBBB", width, height, 8, 2, 0, 0, 0)
    idat = zlib.compress(bytes(raw), level=6)
    return b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", ihdr) + chunk(b"IDAT", idat) + chunk(b"IEND", b"")


def _fallback_png(out_path: str, width: int, height: int, ys: list[float], chart: str) -> None:
    rgb = bytearray([255] * (width * height * 3))

    def set_px(px: int, py: int, r: int, g: int, b: int) -> None:
        if 0 <= px < width and 0 <= py < height:
            idx = (py * width + px) * 3
            rgb[idx] = r
            rgb[idx + 1] = g
            rgb[idx + 2] = b

    # Axes border
    for x in range(40, width - 20):
        set_px(x, height - 40, 40, 40, 40)
    for y in range(20, height - 40):
        set_px(40, y, 40, 40, 40)

    if ys:
        ymin = min(ys)
        ymax = max(ys)
        span = (ymax - ymin) if ymax != ymin else 1.0
        n = max(1, len(ys))
        plot_w = width - 80
        plot_h = height - 80
        prev = None
        for i, val in enumerate(ys):
            cx = 50 + (plot_w // 2 if n == 1 else int(i * plot_w / max(1, n - 1)))
            cy = height - 50 - int(((val - ymin) / span) * plot_h)
            if chart == "bar":
                bw = max(6, plot_w // max(2, n * 2))
                for bx in range(cx - bw // 2, cx + bw // 2):
                    for by in range(cy, height - 40):
                        set_px(bx, by, 38, 92, 168)
            else:
                for dx in range(-3, 4):
                    for dy in range(-3, 4):
                        set_px(cx + dx, cy + dy, 38, 92, 168)
                if prev is not None and chart == "line":
                    x0, y0 = prev
                    steps = max(abs(cx - x0), abs(cy - y0), 1)
                    for s in range(steps + 1):
                        lx = x0 + int((cx - x0) * s / steps)
                        ly = y0 + int((cy - y0) * s / steps)
                        set_px(lx, ly, 38, 92, 168)
                prev = (cx, cy)

    png_bytes = _encode_rgb_png(width, height, rgb)
    parent = os.path.dirname(os.path.abspath(out_path))
    if parent:
        os.makedirs(parent, exist_ok=True)
    with open(out_path, "wb") as handle:
        handle.write(png_bytes)


def execute_plot_plan(params: dict, body: bytes):
    chart = params.get("chart")
    if chart not in ("line", "scatter", "bar", "hist"):
        raise PlanError(f"py-plan: unknown plot chart {chart!r}")
    out_path = params.get("out_path")
    if not isinstance(out_path, str) or not out_path:
        raise PlanError("py-plan: plot save requires an output path")
    ops = params.get("ops")
    if not isinstance(ops, list):
        raise PlanError("py-plan: plot missing frame ops")

    frame = _build_frame(ops)
    x_col = params.get("x") or (frame.columns[0] if frame.columns else None)
    y_col = params.get("y") or (frame.columns[-1] if frame.columns else None)
    xs = frame.get_column(x_col).to_list() if x_col and x_col in frame.columns else []
    ys_raw = frame.get_column(y_col).to_list() if y_col and y_col in frame.columns else []
    ys = [float(v) for v in ys_raw if isinstance(v, (int, float))]

    width, height = 640, 480
    rendered_with_mpl = False
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt

        fig, ax = plt.subplots(figsize=(6.4, 4.8), dpi=100)
        try:
            if chart == "line":
                ax.plot(xs if len(xs) == len(ys_raw) else range(len(ys)), ys)
            elif chart == "scatter":
                ax.scatter(xs if len(xs) == len(ys_raw) else range(len(ys)), ys)
            elif chart == "bar":
                ax.bar(xs if len(xs) == len(ys_raw) else range(len(ys)), ys)
            else:
                ax.hist(ys)
            if params.get("title"):
                ax.set_title(str(params["title"]))
            if params.get("legend"):
                ax.legend([str(y_col or "series")])
            parent = os.path.dirname(os.path.abspath(out_path))
            if parent:
                os.makedirs(parent, exist_ok=True)
            fig.savefig(out_path, format="png")
        finally:
            plt.close(fig)
        rendered_with_mpl = True
    except Exception:
        pass

    if not rendered_with_mpl:
        _fallback_png(out_path, width, height, ys, chart)

    # Zero frame rows are returned to the host: body stays empty.
    return {"dtype": "plot", "saved": out_path, "width": width, "height": height}, b""
