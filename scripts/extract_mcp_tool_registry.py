"""Bootstrap: extract TOOL_REGISTRY from vox-mcp into canonical YAML; prefer editing YAML + build.rs after."""
from __future__ import annotations

import json
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
        "# Re-bootstrap from legacy mod.rs: python scripts/extract_mcp_tool_registry.py write",
        "version: 1",
        "tools:",
    ]
    for name, desc in pairs:
        lines.append(f"  - name: {json.dumps(name)}")
        lines.append(f"    description: {json.dumps(desc)}")
    return "\n".join(lines) + "\n"


def main() -> None:
    root = Path(__file__).resolve().parents[1]
    yaml_path = root / "contracts/mcp/tool-registry.canonical.yaml"
    mod_path = root / "crates/vox-mcp/src/tools/mod.rs"
    text = mod_path.read_text(encoding="utf-8")
    pairs = parse_registry(text)
    print(f"found {len(pairs)} tools", file=sys.stderr)

    if len(sys.argv) < 2 or sys.argv[1] != "write":
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
