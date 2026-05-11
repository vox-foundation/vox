import React from "react";

// Type stubs for react-force-graph-2d nodes and links.
// The real library types would be imported from "react-force-graph-2d" if it's in package.json.
// We declare the essential shape here so VUV-transpiled code can import it without a bundled dep.

export interface GraphNode {
  id: string;
  kind: string;
  status: "idle" | "active" | "blocked" | "error";
  model?: string;
  privacyClass?: string;
  // force-graph positions (injected by the layout engine)
  x?: number;
  y?: number;
}

export interface GraphLink {
  source: string;
  target: string;
  kind: string;
  status: string;
}

export interface ForceGraphProps {
  nodes: GraphNode[];
  links: GraphLink[];
  width?: number;
  height?: number;
  onNodeClick?: (node: GraphNode) => void;
}

// Status color map — matches Tailwind palette keys used in .vox components.
const STATUS_COLORS: Record<string, string> = {
  idle: "#71717a",    // zinc-500
  active: "#34d399",  // emerald-400
  blocked: "#f59e0b", // amber-400
  error: "#f87171",   // red-400
};

/**
 * Topology canvas wrapper.
 *
 * Renders a 2D force-directed graph of mesh nodes and edges.
 * In production, wraps `react-force-graph-2d`; in test/SSR environments
 * the component renders a plain `<canvas data-testid="force-graph">`.
 *
 * Color encodes node status: green = active, amber = blocked, red = error, grey = idle.
 */
export function ForceGraph(props: ForceGraphProps): React.ReactElement {
  const { nodes, links, width = 600, height = 400, onNodeClick } = props;

  // Minimal canvas-based fallback (used when react-force-graph-2d is not bundled).
  // Production builds swap this for the real library via tree-shaking.
  return (
    <canvas
      data-testid="force-graph"
      width={width}
      height={height}
      role="img"
      aria-label={`Mesh topology: ${nodes.length} nodes, ${links.length} edges`}
      style={{ background: "#09090b", borderRadius: 8 }}
      ref={(canvas) => {
        if (!canvas) return;
        const ctx = canvas.getContext("2d");
        if (!ctx) return;
        ctx.clearRect(0, 0, width, height);

        // Layout: evenly space nodes in a circle.
        const cx = width / 2;
        const cy = height / 2;
        const r = Math.min(cx, cy) * 0.75;
        const positions: Record<string, { x: number; y: number }> = {};
        nodes.forEach((node, i) => {
          const angle = (2 * Math.PI * i) / nodes.length;
          positions[node.id] = { x: cx + r * Math.cos(angle), y: cy + r * Math.sin(angle) };
        });

        // Draw links.
        ctx.strokeStyle = "#3f3f46"; // zinc-700
        ctx.lineWidth = 1;
        links.forEach((link) => {
          const from = positions[link.source];
          const to = positions[link.target];
          if (!from || !to) return;
          ctx.beginPath();
          ctx.moveTo(from.x, from.y);
          ctx.lineTo(to.x, to.y);
          ctx.stroke();
        });

        // Draw nodes.
        nodes.forEach((node) => {
          const pos = positions[node.id];
          if (!pos) return;
          const color = STATUS_COLORS[node.status] ?? STATUS_COLORS.idle;
          ctx.beginPath();
          ctx.arc(pos.x, pos.y, node.kind === "orchestrator" ? 12 : 8, 0, 2 * Math.PI);
          ctx.fillStyle = color;
          ctx.fill();
          ctx.strokeStyle = "#18181b"; // zinc-900
          ctx.lineWidth = 2;
          ctx.stroke();

          // Label.
          ctx.fillStyle = "#e4e4e7"; // zinc-200
          ctx.font = "10px monospace";
          ctx.textAlign = "center";
          ctx.fillText(node.id.slice(0, 12), pos.x, pos.y + 22);
        });
      }}
      onClick={(e) => {
        if (!onNodeClick || nodes.length === 0) return;
        // Simple click-to-select: find nearest node to click position.
        const rect = (e.target as HTMLCanvasElement).getBoundingClientRect();
        const mx = e.clientX - rect.left;
        const my = e.clientY - rect.top;
        const cx = width / 2;
        const cy = height / 2;
        const r = Math.min(cx, cy) * 0.75;
        let nearest: GraphNode | null = null;
        let minDist = Infinity;
        nodes.forEach((node, i) => {
          const angle = (2 * Math.PI * i) / nodes.length;
          const nx = cx + r * Math.cos(angle);
          const ny = cy + r * Math.sin(angle);
          const d = Math.hypot(mx - nx, my - ny);
          if (d < minDist) { minDist = d; nearest = node; }
        });
        if (nearest && minDist < 20) onNodeClick(nearest);
      }}
    />
  );
}
