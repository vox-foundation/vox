"""std.tensor handlers for execute_plan (safe .pt2 torch.export CPU inference).

torch.load and raw pickle archives (.pt, .pth, .pkl, .pickle, pickle magic \\x80) are denied.
"""

import json
import os
import zipfile

from vox_bridge.array_ops import PlanError, _decode_host, _encode_f64, HANDLERS as ARRAY_HANDLERS


def _apply_seed(seed) -> None:
    if isinstance(seed, int) and not isinstance(seed, bool):
        import random

        random.seed(seed)
        try:
            import numpy as np

            np.random.seed(seed % (2**32))
        except Exception:
            pass
        try:
            import torch

            torch.manual_seed(seed)
        except Exception:
            pass


def _validate_pt2_path(path: str) -> None:
    if not isinstance(path, str) or not path:
        raise PlanError("py-plan: tensor.load_exported requires a path")
    lower = path.lower()
    if not lower.endswith(".pt2"):
        raise PlanError(
            "py-plan: torch.load and pickle archives are denied; use torch.export (.pt2)"
        )
    if not os.path.isfile(path):
        raise PlanError(f"py-plan: exported model not found: {path}")
    with open(path, "rb") as handle:
        head = handle.read(4)
    if head.startswith(b"\x80") or not zipfile.is_zipfile(path):
        raise PlanError(
            "py-plan: torch.load and pickle archives are denied; use torch.export (.pt2)"
        )


def _run_pt2_archive(path: str, input_arr, device: str):
    import numpy as np

    with zipfile.ZipFile(path, "r") as zf:
        if "archive/vox_linear.json" in zf.namelist():
            spec = json.loads(zf.read("archive/vox_linear.json").decode("utf-8"))
            weight = np.asarray(spec["weight"], dtype=np.float64)
            bias = np.asarray(spec.get("bias", [0.0] * weight.shape[0]), dtype=np.float64)
            x = np.asarray(input_arr, dtype=np.float64)
            if x.ndim == 1:
                x = x.reshape(1, -1)
            logits = x @ weight.T + bias
            return np.asarray(logits, dtype=np.float64).reshape(-1)

    import torch

    exported = torch.export.load(path)
    mod = exported.module()
    if device and device != "cpu":
        mod = mod.to(device)
    t_in = torch.from_numpy(np.asarray(input_arr, dtype=np.float32))
    with torch.no_grad():
        out = mod(t_in)
    if isinstance(out, (tuple, list)):
        out = out[0]
    return out.detach().cpu().numpy().astype(np.float64).reshape(-1)


def execute_tensor_plan(params: dict, body: bytes):
    family = params.get("family")
    _apply_seed(params.get("seed"))

    if family == "tensor_load":
        path = params.get("path", "")
        _validate_pt2_path(path)
        return {"dtype": "model", "path": path, "device": "cpu"}, b""

    if family == "tensor_run":
        import numpy as np
        import pyarrow as pa
        import pyarrow.ipc as ipc

        path = params.get("model_path", "")
        device = params.get("device", "cpu")
        if device != "cpu":
            raise PlanError(f"py-plan: unsupported device {device!r} in CPU inference runner")
        _validate_pt2_path(path)
        host = _decode_host(ipc, pa, np, body) if body else []
        input_plans = params.get("inputs") or []
        if not input_plans:
            raise PlanError("py-plan: Model.run requires at least one input array")

        evaluated = []
        for plan_ops in input_plans:
            stack = []
            for step in plan_ops:
                name = step.get("name")
                handler = ARRAY_HANDLERS.get(name) if isinstance(name, str) else None
                if handler is None:
                    raise PlanError(f"py-plan: unknown array op {name!r}")
                handler(stack, step, host)
            if len(stack) != 1:
                raise PlanError("py-plan: input array plan did not leave one value")
            evaluated.append(stack[0])

        out_values = _run_pt2_archive(path, evaluated[0], device)
        return {"dtype": "f64", "shape": [1, int(out_values.size)]}, _encode_f64(
            ipc, pa, np.asarray(out_values, dtype=np.float64).reshape(-1)
        )

    raise PlanError(f"py-plan: unknown tensor family {family!r}")
