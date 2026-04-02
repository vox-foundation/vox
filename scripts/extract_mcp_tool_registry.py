"""Legacy migration helper: extract TOOL_REGISTRY from vox-mcp into canonical YAML.

This script is intentionally gated because contracts/mcp/tool-registry.canonical.yaml is the
active SSOT. Prefer editing YAML directly and regenerating Rust via vox-mcp-registry/build.rs.
"""
from __future__ import annotations

import json
import os
import re
import sys
from pathlib import Path


def parse_registry(mod_rs: str) -> list[tuple[str, str]]:
    start = mod_rs.index("pub const TOOL_REGISTRY")
    end = mod_rs.index("];", start)
    block = mod_rs[start:end]
    # Allow optional trailing comma after the description (Rustfmt multi-line tuples).
    pat = re.compile(
        r'\(\s*"([^"]+)"\s*,\s*"((?:[^"\\]|\\.)*)"\s*,?\s*\)',
        re.DOTALL,
    )
    pairs = [(m.group(1), m.group(2)) for m in pat.finditer(block)]
    if not pairs:
        raise SystemExit("no tools parsed from TOOL_REGISTRY")
    return pairs


def emit_yaml(pairs: list[tuple[str, str]]) -> str:
    lines = [
        "# Canonical MCP tool names + descriptions (SSOT).",
        "# Edited here; Rust builds via crates/vox-mcp-registry/build.rs.",
        "# Legacy recovery from mod.rs (disabled by default):",
        "# VOX_ALLOW_LEGACY_MCP_EXTRACT=1 python scripts/extract_mcp_tool_registry.py --allow-legacy write",
        "# After write, run: python scripts/mcp_registry_fill_product_lanes.py",
        "version: 1",
        "tools:",
    ]
    for name, desc in pairs:
        lines.append(f"  - name: {json.dumps(name)}")
        lines.append(f"    description: {json.dumps(desc)}")
    return "\n".join(lines) + "\n"


def main() -> None:
    allow_legacy = (
        len(sys.argv) >= 2
        and sys.argv[1] == "--allow-legacy"
        and os.environ.get("VOX_ALLOW_LEGACY_MCP_EXTRACT", "").strip() == "1"
    )
    if not allow_legacy:
        raise SystemExit(
            "legacy tool disabled by default. "
            "If you are performing one-time migration recovery, run with "
            "VOX_ALLOW_LEGACY_MCP_EXTRACT=1 and pass --allow-legacy before other args."
        )

    argv = [a for a in sys.argv[1:] if a != "--allow-legacy"]

    root = Path(__file__).resolve().parents[1]
    yaml_path = root / "contracts/mcp/tool-registry.canonical.yaml"
    mod_path = root / "crates/vox-mcp/src/tools/mod.rs"
    text = mod_path.read_text(encoding="utf-8")
    pairs = parse_registry(text)
    print(f"found {len(pairs)} tools", file=sys.stderr)

    if len(argv) < 1 or argv[0] != "write":
        for name, desc in pairs:
            if "\n" in desc:
                raise SystemExit(f"multiline desc for {name!r}")
        print("ok (dry run); pass 'write' to update YAML + generated.rs")
        return

    yaml_path.parent.mkdir(parents=True, exist_ok=True)
    yaml_path.write_text(emit_yaml(pairs), encoding="utf-8", newline="\n")
    print(f"wrote {yaml_path}", file=sys.stderr)


if __name__ == "__main__":
    main()
