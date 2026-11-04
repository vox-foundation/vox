"""std.frame handlers for one execute_plan.

polars is imported inside execute_plan, after the worker has moved RPC off
stdout. Op dispatch is this dict. Unknown names fail closed.
"""


class PlanError(Exception):
    """Plan rejected before polars runs. The message starts with py-plan:."""


def scan_csv(lf, step, exprs):
    path = step.get("path")
    if not isinstance(path, str) or not path:
        raise PlanError(f"py-plan: scan_csv path {path!r}")
    import polars as pl

    return pl.scan_csv(path, glob=False)


def col(lf, step, exprs):
    name = step.get("column")
    if not isinstance(name, str) or not name:
        raise PlanError(f"py-plan: col name {name!r}")
    import polars as pl

    exprs.append(pl.col(name))
    return lf


def _build_expr(node):
    import polars as pl

    if not isinstance(node, dict):
        raise PlanError("py-plan: expr is not an object")
    name = node.get("name")
    if name == "col":
        column = node.get("column")
        if not isinstance(column, str) or not column:
            raise PlanError(f"py-plan: col name {column!r}")
        return pl.col(column)
    child = node.get("expr")
    if child is None:
        raise PlanError(f"py-plan: {name} missing child expr")
    base = _build_expr(child)
    literal = node.get("literal")
    if name == "eq":
        return base.eq(literal)
    if name == "neq":
        return base.ne(literal)
    if name == "gt":
        return base.gt(_coerce_literal(literal))
    if name == "gte":
        return base.ge(_coerce_literal(literal))
    if name == "lt":
        return base.lt(_coerce_literal(literal))
    if name == "lte":
        return base.le(_coerce_literal(literal))
    if name == "and":
        rhs = node.get("rhs")
        if rhs is None:
            raise PlanError("py-plan: and missing rhs")
        return base.and_(_build_expr(rhs))
    if name == "or":
        rhs = node.get("rhs")
        if rhs is None:
            raise PlanError("py-plan: or missing rhs")
        return base.or_(_build_expr(rhs))
    if name == "sum":
        out = base.sum()
    elif name == "count":
        out = base.count()
    elif name == "mean":
        out = base.mean()
    elif name == "min":
        out = base.min()
    elif name == "max":
        out = base.max()
    else:
        raise PlanError(f"py-plan: unknown op {name!r}")
    alias = node.get("alias")
    if isinstance(alias, str) and alias:
        out = out.alias(alias)
    return out


def _coerce_literal(literal):
    if isinstance(literal, bool):
        return literal
    if isinstance(literal, (int, float)):
        return literal
    if isinstance(literal, str):
        try:
            if "." in literal:
                return float(literal)
            return int(literal)
        except ValueError:
            return literal
    return literal


def filter(lf, step, exprs):
    node = step.get("expr")
    if node is None:
        raise PlanError("py-plan: filter missing expr")
    return lf.filter(_build_expr(node))


def group_by(lf, step, exprs):
    keys = step.get("keys")
    if not isinstance(keys, list) or not keys:
        raise PlanError(f"py-plan: group_by keys {keys!r}")
    return lf.group_by(keys)


def agg(lf, step, exprs):
    nodes = step.get("exprs")
    if not isinstance(nodes, list) or not nodes:
        raise PlanError("py-plan: agg missing exprs")
    built = [_build_expr(node) for node in nodes]
    return lf.agg(built)


def sort(lf, step, exprs):
    column = step.get("column")
    if not isinstance(column, str) or not column:
        raise PlanError(f"py-plan: sort column {column!r}")
    nulls = step.get("nulls")
    if nulls not in ("first", "last"):
        raise PlanError(f"py-plan: sort nulls {nulls!r}")
    nulls_last = True if nulls == "last" else False
    return lf.sort(column, nulls_last=nulls_last)


def sum(lf, step, exprs):
    raise PlanError("py-plan: sum is an expression op, not a frame step")


def count(lf, step, exprs):
    raise PlanError("py-plan: count is an expression op, not a frame step")


def mean(lf, step, exprs):
    raise PlanError("py-plan: mean is an expression op, not a frame step")


def min(lf, step, exprs):
    raise PlanError("py-plan: min is an expression op, not a frame step")


def max(lf, step, exprs):
    raise PlanError("py-plan: max is an expression op, not a frame step")


def eq(lf, step, exprs):
    raise PlanError("py-plan: eq is an expression op, not a frame step")


def neq(lf, step, exprs):
    raise PlanError("py-plan: neq is an expression op, not a frame step")


def gt(lf, step, exprs):
    raise PlanError("py-plan: gt is an expression op, not a frame step")


def gte(lf, step, exprs):
    raise PlanError("py-plan: gte is an expression op, not a frame step")


def lt(lf, step, exprs):
    raise PlanError("py-plan: lt is an expression op, not a frame step")


def lte(lf, step, exprs):
    raise PlanError("py-plan: lte is an expression op, not a frame step")


def and_(lf, step, exprs):
    raise PlanError("py-plan: and is an expression op, not a frame step")


def or_(lf, step, exprs):
    raise PlanError("py-plan: or is an expression op, not a frame step")


def collect(lf, step, exprs):
    if lf is None:
        raise PlanError("py-plan: collect on empty plan")
    import polars as pl

    frame = lf.collect()
    for column in frame.columns:
        series = frame.get_column(column)
        if series.null_count() > 0:
            raise PlanError(
                f"py-plan: scan_csv must drop or fill nulls in column {column}"
            )
        dtype = series.dtype
        if (
            hasattr(dtype, "time_zone")
            and dtype.time_zone is None
            and str(dtype).startswith("Datetime")
        ):
            raise PlanError(
                'py-plan: naive timestamp; use .replace_time_zone("UTC")'
            )
        if dtype == pl.Struct or str(dtype).startswith("Struct"):
            raise PlanError(f"py-plan: unsupported struct column {column}")
        if str(dtype).startswith("List") or str(dtype).startswith("Array"):
            raise PlanError(f"py-plan: unsupported nested column {column}")
    return frame


HANDLERS = {
    "scan_csv": scan_csv,
    "col": col,
    "filter": filter,
    "group_by": group_by,
    "agg": agg,
    "sort": sort,
    "sum": sum,
    "count": count,
    "mean": mean,
    "min": min,
    "max": max,
    "eq": eq,
    "neq": neq,
    "gt": gt,
    "gte": gte,
    "lt": lt,
    "lte": lte,
    "and": and_,
    "or": or_,
    "collect": collect,
}


def execute_plan(params, body: bytes):
    """Run the frame plan JSON in `params`. Returns (value, body)."""
    import sys

    import polars as pl
    import pyarrow as pa
    import pyarrow.ipc as ipc

    if not isinstance(params, dict):
        raise PlanError("py-plan: params must be the plan")
    if params.get("family") != "frame":
        raise PlanError("py-plan: family must be frame")
    ops = params.get("ops")
    if not isinstance(ops, list):
        raise PlanError("py-plan: plan has no ops")
    sys.stderr.write(
        "vox_bridge frame plan "
        f"payload={params!r} body_len={len(body) if body else 0}\n"
    )
    sys.stderr.flush()
    lf = None
    exprs = []
    for step in ops:
        if not isinstance(step, dict):
            raise PlanError("py-plan: op is not an object")
        name = step.get("name")
        handler = HANDLERS.get(name) if isinstance(name, str) else None
        if handler is None:
            raise PlanError(f"py-plan: unknown op {name!r}")
        lf = handler(lf, step, exprs)
    if lf is None:
        raise PlanError("py-plan: plan did not build a frame")
    if not isinstance(lf, pl.DataFrame):
        lf = collect(lf, {"name": "collect"}, exprs)
    table = lf.to_arrow()
    sink = pa.BufferOutputStream()
    writer = ipc.new_stream(sink, table.schema)
    writer.write_table(table)
    writer.close()
    return {"dtype": "frame"}, sink.getvalue().to_pybytes()
