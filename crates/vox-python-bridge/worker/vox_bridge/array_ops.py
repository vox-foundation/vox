"""std.array handlers for one execute_plan.

numpy and pyarrow are imported inside the handler, after the worker has
moved RPC off stdout. Op dispatch is this dict. Unknown names fail closed.
"""


class PlanError(Exception):
    """Plan rejected before numpy runs. The message starts with py-plan:."""


def from_list(stack, step, host):
    stack.append(host.copy())


def astype(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: astype on empty stack")
    dtype = step.get("dtype")
    if dtype != "f64":
        raise PlanError(f"py-plan: astype dtype {dtype!r}")
    stack[-1] = np.asarray(stack[-1], dtype=np.float64)


def add(stack, step, host):
    import numpy as np

    if len(stack) < 2:
        raise PlanError("py-plan: add needs two values")
    rhs = stack.pop()
    lhs = stack.pop()
    stack.append(np.asarray(lhs + rhs, dtype=np.float64))


def sub(stack, step, host):
    import numpy as np

    if len(stack) < 2:
        raise PlanError("py-plan: sub needs two values")
    rhs = stack.pop()
    lhs = stack.pop()
    stack.append(np.asarray(lhs - rhs, dtype=np.float64))


def mul(stack, step, host):
    import numpy as np

    if len(stack) < 2:
        raise PlanError("py-plan: mul needs two values")
    rhs = stack.pop()
    lhs = stack.pop()
    stack.append(np.asarray(lhs * rhs, dtype=np.float64))


def div(stack, step, host):
    import numpy as np

    if len(stack) < 2:
        raise PlanError("py-plan: div needs two values")
    rhs = stack.pop()
    lhs = stack.pop()
    stack.append(np.asarray(lhs / rhs, dtype=np.float64))


def mean(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: mean on empty stack")
    stack[-1] = np.asarray(np.mean(stack[-1]), dtype=np.float64)


def std(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: std on empty stack")
    correction = step.get("correction")
    if isinstance(correction, bool) or not isinstance(correction, int):
        raise PlanError("py-plan: std requires correction")
    # The Vox name is correction. Do not pass ddof.
    stack[-1] = np.asarray(np.std(stack[-1], correction=correction), dtype=np.float64)


def sort(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: sort on empty stack")
    nulls = step.get("nulls")
    if nulls not in ("first", "last"):
        raise PlanError(f"py-plan: sort nulls {nulls!r}")
    values = np.asarray(stack[-1], dtype=np.float64).reshape(-1)
    # np.sort has no nulls argument. Partition, sort the finite values, then join.
    finite_mask = ~np.isnan(values)
    ordered = np.sort(values[finite_mask])
    nans = values[~finite_mask]
    if nulls == "first":
        out = np.concatenate((nans, ordered))
    else:
        out = np.concatenate((ordered, nans))
    stack[-1] = np.asarray(out, dtype=np.float64)


def to_list(stack, step, host):
    if not stack:
        raise PlanError("py-plan: to_list on empty stack")


def item(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: item on empty stack")
    values = np.asarray(stack[-1], dtype=np.float64).reshape(-1)
    if values.size != 1:
        raise PlanError("py-plan: item requires one value")
    stack[-1] = np.asarray(values, dtype=np.float64)


def shape(stack, step, host):
    import numpy as np

    if not stack:
        raise PlanError("py-plan: shape on empty stack")
    values = np.asarray(stack[-1])
    stack[-1] = np.asarray(values.shape, dtype=np.float64)


HANDLERS = {
    "from_list": from_list,
    "astype": astype,
    "add": add,
    "sub": sub,
    "mul": mul,
    "div": div,
    "mean": mean,
    "std": std,
    "sort": sort,
    "to_list": to_list,
    "item": item,
    "shape": shape,
}


def execute_plan(params, body: bytes):
    """Run the plan JSON in `params` against the Arrow body. Returns (value, body)."""
    import sys

    import numpy as np
    import pyarrow as pa
    import pyarrow.ipc as ipc

    if not isinstance(params, dict):
        raise PlanError("py-plan: params must be the plan")
    ops = params.get("ops")
    if not isinstance(ops, list):
        raise PlanError("py-plan: plan has no ops")
    host = _decode_host(ipc, pa, np, body)
    # Payload echo for the plan parser. This stays on the worker stderr the
    # host already captured. It is not an RPC method line.
    sys.stderr.write(
        "vox_bridge array plan "
        f"payload={params!r} body_len={len(body) if body else 0} "
        f"host_dtype={getattr(host, 'dtype', type(host).__name__)}\n"
    )
    sys.stderr.flush()
    stack = []
    for step in ops:
        if not isinstance(step, dict):
            raise PlanError("py-plan: op is not an object")
        name = step.get("name")
        handler = HANDLERS.get(name) if isinstance(name, str) else None
        if handler is None:
            raise PlanError(f"py-plan: unknown op {name!r}")
        handler(stack, step, host)
    if len(stack) != 1:
        raise PlanError("py-plan: plan did not leave one value")
    values = np.asarray(stack[0]).reshape(-1)
    if values.dtype == np.int64:
        return {"dtype": "i64"}, _encode_i64(ipc, pa, values)
    values = np.asarray(values, dtype=np.float64)
    return {"dtype": "f64"}, _encode_f64(ipc, pa, values)


def _decode_host(ipc, pa, np, body: bytes):
    if not body:
        raise PlanError("py-plan: missing arrow body")
    reader = ipc.open_stream(pa.py_buffer(body))
    batch = reader.read_next_batch()
    if batch is None or batch.num_columns < 1:
        raise PlanError("py-plan: arrow body has no column")
    column = batch.column(0)
    if pa.types.is_int64(column.type):
        return np.asarray(column.to_numpy(zero_copy_only=False), dtype=np.int64)
    if pa.types.is_floating(column.type):
        return np.asarray(column.to_numpy(zero_copy_only=False), dtype=np.float64)
    raise PlanError(f"py-plan: unsupported host column {column.type}")


def _encode_i64(ipc, pa, values):
    array = pa.array(values, type=pa.int64())
    batch = pa.record_batch([array], names=["values"])
    sink = pa.BufferOutputStream()
    with ipc.new_stream(sink, batch.schema) as writer:
        writer.write_batch(batch)
    return sink.getvalue().to_pybytes()


def _encode_f64(ipc, pa, values):
    array = pa.array(values, type=pa.float64())
    batch = pa.record_batch([array], names=["values"])
    sink = pa.BufferOutputStream()
    with ipc.new_stream(sink, batch.schema) as writer:
        writer.write_batch(batch)
    return sink.getvalue().to_pybytes()
