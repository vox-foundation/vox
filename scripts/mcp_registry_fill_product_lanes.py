#!/usr/bin/env python3
"""One-shot / idempotent: ensure every MCP tool in tool-registry.canonical.yaml has product_lane.

Bell-curve lanes match CLI SSOT: app, workflow, ai, interop, data, platform.
Run from repo root: python scripts/mcp_registry_fill_product_lanes.py
"""
from __future__ import annotations

import sys
from pathlib import Path

import yaml

KNOWN = frozenset({"app", "workflow", "ai", "interop", "data", "platform"})

APP_EXACT = frozenset(
    {
        "vox_validate_file",
        "vox_run_tests",
        "vox_check_workspace",
        "vox_test_all",
        "vox_build_crate",
        "vox_lint_crate",
        "vox_coverage_report",
        "vox_language_surface",
        "vox_pipeline_status",
        "vox_decorator_registry",
        "vox_builtin_registry",
        "vox_workspace_modules",
        "vox_compiler::ast_inspect",
    }
)


def infer_lane(name: str) -> str:
    if name.startswith("vox_openclaw_") or name.startswith("vox_skill_"):
        return "interop"
    if name.startswith("vox_scientia_"):
        return "data"
    if (
        name.startswith("vox_db_")
        or name.startswith("vox_preference_")
        or name.startswith("vox_knowledge_")
    ):
        return "data"
    if name.startswith("vox_news_"):
        return "data"
    if name.startswith("vox_populi_"):
        return "workflow"
    if name.startswith("vox_git_") or name.startswith("vox_repo_index_"):
        return "app"
    if name in APP_EXACT:
        return "app"
    if name.startswith("vox_toestub_") or name.startswith("vox_benchmark_"):
        return "platform"
    if name.startswith("vox_config_"):
        return "platform"
    return "ai"


def main() -> int:
    root = Path(__file__).resolve().parents[1]
    path = root / "contracts/mcp/tool-registry.canonical.yaml"
    raw = path.read_text(encoding="utf-8")
    preamble_lines: list[str] = []
    for line in raw.splitlines(keepends=True):
        if line.startswith("#") or (line.strip() == "" and preamble_lines and preamble_lines[-1].startswith("#")):
            preamble_lines.append(line)
            continue
        break
    preamble = "".join(preamble_lines).rstrip("\n")
    if preamble:
        preamble += "\n"
    doc = yaml.safe_load(raw)
    tools = doc.get("tools") or []
    changed = 0
    for t in tools:
        name = t.get("name")
        if not name:
            print("tool entry missing name", file=sys.stderr)
            return 1
        lane = t.get("product_lane")
        want = infer_lane(name)
        if lane is None:
            t["product_lane"] = want
            changed += 1
        elif lane != want:
            print(
                f"warn: {name}: yaml product_lane={lane!r} differs from infer_lane={want!r} (leaving yaml)",
                file=sys.stderr,
            )
        if t["product_lane"] not in KNOWN:
            print(f"invalid product_lane for {name}: {t['product_lane']}", file=sys.stderr)
            return 1
    out = yaml.safe_dump(
        doc,
        allow_unicode=True,
        default_flow_style=False,
        sort_keys=False,
        width=120,
    )
    path.write_text(f"{preamble}{out}", encoding="utf-8", newline="\n")
    print(f"wrote {path} ({changed} new product_lane fields)")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
