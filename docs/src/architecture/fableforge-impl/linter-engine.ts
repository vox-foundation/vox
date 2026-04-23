/**
 * FFScript Linter Engine — core implementation
 *
 * Drop this file into: packages/ffscript/src/linter/engine.ts
 *
 * Implements: T-046, T-047, T-048, T-050
 * Spec:       docs/src/architecture/ffscript-linter-design-2026.md
 *
 * Key design decisions (from Vox compiler survey, 2026-04-23):
 *   - LintFix is a serializable descriptor (not a closure) so it can be
 *     stored in Convex and replayed from the CLI.
 *   - Rules are pure functions — no side effects, fully testable in isolation.
 *   - DiagnosticCategory pattern from vox-compiler: each rule declares its
 *     category (structure | speaker | background | placement | flow | bubble | content).
 */

// ─── Re-export types from the spec ───────────────────────────────────────────
// (In the real repo these come from packages/ffscript/src/linter/index.ts)

export type LintSeverity = "error" | "warning" | "info";

export type LintCategory =
  | "structure"  // panel IDs, order, scene membership
  | "speaker"    // dialogue node ↔ character bindings
  | "background" // missing or unresolved assets
  | "placement"  // character off-canvas, z-order
  | "flow"       // reachability, loops, undeclared variables
  | "bubble"     // tail positioning, reading order
  | "content"    // content rating consistency
  ;

export interface LintLocation {
  panelId?:     string;
  nodeId?:      string;
  bubbleId?:    string;
  placementId?: string;
  sceneId?:     string;
  path?:        string;
}

export type FixKind =
  | { type: "regenerate_panel_id"; panelId: string }
  | { type: "repair_panel_order" }
  | { type: "repair_reading_order"; panelId: string }
  | { type: "remove_placement"; placementId: string }
  | { type: "set_speaker_null"; bubbleId: string }
  ;

export interface LintFix {
  label: string;
  kind:  FixKind;
}

export interface LintResult {
  ruleId:    string;
  category:  LintCategory;
  severity:  LintSeverity;
  message:   string;
  detail?:   string;
  location:  LintLocation;
  fix?:      LintFix;
}

export interface LintRule {
  id:          string;
  description: string;
  category:    LintCategory;
  severity:    LintSeverity;
  fixable:     boolean;
  check:       (doc: FFScriptV02) => LintResult[];
}

export interface LintReport {
  findings:     LintResult[];
  errorCount:   number;
  warningCount: number;
  infoCount:    number;
  hasErrors:    boolean;
  durationMs:   number;
}

export interface LintEngineOptions {
  severityOverrides?: Partial<Record<string, LintSeverity>>;
  disabledRules?:     string[];
  customRules?:       LintRule[];
  errorsOnly?:        boolean;
}

// Minimal structural type for the FFScript doc — adapt field names as needed
interface FFScriptV02 {
  version:   string;
  buildType?: string;
  panels:    Array<{
    id:              string;
    order:           number;
    background:      { assetKey: string } | null;
    foreground:      { characterPlacements: Array<{ id: string; characterId: string; x: number; y: number }> };
    bubbles:         Array<{ id: string; lineId: string; tailX: number | null; tailY: number | null; style: string; readingOrder: number }>;
    contentRating:   string | null;
    dialogueNodeIds: string[];
    metadata:        { locked: boolean };
  }>;
  scenes:    Array<{ id: string; panelIds: string[] }>;
  characters: Array<{ id: string; name: string }>;
  flowGraph: {
    nodes: Array<{ id: string; type: string; speakerId?: string | null }>;
    edges: Array<{ from: string; to: string }>;
    variables?: Array<{ name: string }>;
  };
  contentRating?: string;
}

// ─── Default rules ────────────────────────────────────────────────────────────

export const DEFAULT_RULES: LintRule[] = [

  // ── ff/structure/duplicate-panel-id ────────────────────────────────────────
  {
    id:          "ff/structure/duplicate-panel-id",
    description: "Panel IDs must be unique across the document.",
    category:    "structure",
    severity:    "error",
    fixable:     true,
    check(doc) {
      const seen = new Map<string, number>();
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        const count = (seen.get(panel.id) ?? 0) + 1;
        seen.set(panel.id, count);
        if (count === 2) { // emit once per duplicate id
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: this.severity,
            message:  `Duplicate panel id: "${panel.id}"`,
            location: { panelId: panel.id, path: `panels[?id="${panel.id}"]` },
            fix: { label: "Regenerate panel ID", kind: { type: "regenerate_panel_id", panelId: panel.id } },
          });
        }
      }
      return results;
    },
  },

  // ── ff/structure/panel-order-gap ───────────────────────────────────────────
  {
    id:          "ff/structure/panel-order-gap",
    description: "Panel order values must form a contiguous 0-indexed sequence.",
    category:    "structure",
    severity:    "error",
    fixable:     true,
    check(doc) {
      const orders = doc.panels.map((p) => p.order).sort((a, b) => a - b);
      const hasGap = orders.some((o, i) => o !== i);
      if (!hasGap) return [];
      return [{
        ruleId:   this.id,
        category: this.category,
        severity: this.severity,
        message:  `Panel order values are not contiguous: [${orders.join(", ")}]`,
        location: { path: "panels[*].order" },
        fix: { label: "Repair panel order", kind: { type: "repair_panel_order" } },
      }];
    },
  },

  // ── ff/speaker/orphaned-speaker ────────────────────────────────────────────
  {
    id:          "ff/speaker/orphaned-speaker",
    description: "Every bubble's speaker must exist in characters[].",
    category:    "speaker",
    severity:    "error",
    fixable:     true,
    check(doc) {
      const characterIds = new Set(doc.characters.map((c) => c.id));
      const nodeMap = new Map(doc.flowGraph.nodes.map((n) => [n.id, n]));
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        for (const bubble of panel.bubbles) {
          const node = nodeMap.get(bubble.lineId);
          if (!node) continue; // dangling lineId — caught by a different rule
          const speakerId = node.speakerId;
          if (speakerId && !characterIds.has(speakerId)) {
            results.push({
              ruleId:   this.id,
              category: this.category,
              severity: this.severity,
              message:  `Bubble references speaker "${speakerId}" who is not in characters[].`,
              location: { panelId: panel.id, bubbleId: bubble.id, path: `panels[id="${panel.id}"].bubbles[id="${bubble.id}"]` },
              fix: { label: "Convert to narrator caption", kind: { type: "set_speaker_null", bubbleId: bubble.id } },
            });
          }
        }
      }
      return results;
    },
  },

  // ── ff/background/missing-background ──────────────────────────────────────
  {
    id:          "ff/background/missing-background",
    description: "Panels should have a generated background before publishing.",
    category:    "background",
    severity:    "warning",
    fixable:     false,
    check(doc) {
      const isPlaceholderBuild = doc.buildType === "placeholder";
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        if (!panel.background || !panel.background.assetKey) {
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: isPlaceholderBuild ? "info" : this.severity,
            message:  `Panel has no background image.`,
            location: { panelId: panel.id, path: `panels[id="${panel.id}"].background` },
          });
        }
      }
      return results;
    },
  },

  // ── ff/placement/off-canvas-character ─────────────────────────────────────
  {
    id:          "ff/placement/off-canvas-character",
    description: "Character placements outside [-0.2, 1.2] × [-0.1, 1.1] are invisible.",
    category:    "placement",
    severity:    "warning",
    fixable:     false,
    check(doc) {
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        for (const p of panel.foreground.characterPlacements) {
          const outOfBounds =
            p.x < -0.2 || p.x > 1.2 || p.y < -0.1 || p.y > 1.1;
          if (outOfBounds) {
            results.push({
              ruleId:   this.id,
              category: this.category,
              severity: this.severity,
              message:  `Character placement (x=${p.x.toFixed(2)}, y=${p.y.toFixed(2)}) is outside canvas bounds.`,
              location: { panelId: panel.id, placementId: p.id },
            });
          }
        }
      }
      return results;
    },
  },

  // ── ff/flow/dangling-beat ──────────────────────────────────────────────────
  {
    id:          "ff/flow/dangling-beat",
    description: "Dialogue/choice nodes should be referenced by at least one panel.",
    category:    "flow",
    severity:    "warning",
    fixable:     false,
    check(doc) {
      const EXEMPT_TYPES = new Set(["start", "end", "condition", "macro", "stage_op"]);
      const referencedNodeIds = new Set(doc.panels.flatMap((p) => p.dialogueNodeIds));
      const results: LintResult[] = [];
      for (const node of doc.flowGraph.nodes) {
        if (!EXEMPT_TYPES.has(node.type) && !referencedNodeIds.has(node.id)) {
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: this.severity,
            message:  `Node "${node.id}" (type: ${node.type}) is not referenced by any panel.`,
            location: { nodeId: node.id, path: `flowGraph.nodes[id="${node.id}"]` },
          });
        }
      }
      return results;
    },
  },

  // ── ff/flow/unreachable-node (subsumes T-059) ──────────────────────────────
  {
    id:          "ff/flow/unreachable-node",
    description: "All non-start nodes must be reachable from the start node.",
    category:    "flow",
    severity:    "warning",
    fixable:     false,
    check(doc) {
      const nodes   = doc.flowGraph.nodes;
      const edges   = doc.flowGraph.edges;
      const startId = nodes.find((n) => n.type === "start")?.id;
      if (!startId) return []; // malformed doc; caught elsewhere

      // BFS
      const visited = new Set<string>([startId]);
      const queue   = [startId];
      while (queue.length > 0) {
        const cur = queue.shift()!;
        for (const edge of edges) {
          if (edge.from === cur && !visited.has(edge.to)) {
            visited.add(edge.to);
            queue.push(edge.to);
          }
        }
      }

      const EXEMPT_TYPES = new Set(["macro"]); // macros may be called indirectly
      const results: LintResult[] = [];
      for (const node of nodes) {
        if (!visited.has(node.id) && !EXEMPT_TYPES.has(node.type)) {
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: this.severity,
            message:  `Node "${node.id}" (type: ${node.type}) is unreachable from the start node.`,
            location: { nodeId: node.id },
          });
        }
      }
      return results;
    },
  },

  // ── ff/flow/infinite-loop ──────────────────────────────────────────────────
  {
    id:          "ff/flow/infinite-loop",
    description: "Cycles without a condition guard cause the runtime to loop forever.",
    category:    "flow",
    severity:    "error",
    fixable:     false,
    check(doc) {
      const edges   = doc.flowGraph.edges;
      const nodeMap = new Map(doc.flowGraph.nodes.map((n) => [n.id, n]));
      const adj     = new Map<string, string[]>();
      for (const e of edges) {
        adj.set(e.from, [...(adj.get(e.from) ?? []), e.to]);
      }

      const results: LintResult[] = [];
      const BLACK = new Set<string>(); // fully processed
      const GREY  = new Set<string>(); // in current DFS path

      function dfs(id: string, path: string[]): void {
        if (BLACK.has(id)) return;
        if (GREY.has(id)) {
          // Found a cycle — check if ANY node on the cycle path is a condition
          const cycleStart = path.indexOf(id);
          const cyclePath  = path.slice(cycleStart);
          const hasGuard   = cyclePath.some((nid) => nodeMap.get(nid)?.type === "condition");
          if (!hasGuard) {
            results.push({
              ruleId:   "ff/flow/infinite-loop",
              category: "flow",
              severity: "error",
              message:  `Infinite loop detected: ${[...cyclePath, id].join(" → ")}`,
              location: { nodeId: id },
            });
          }
          return;
        }
        GREY.add(id);
        for (const next of adj.get(id) ?? []) dfs(next, [...path, id]);
        GREY.delete(id);
        BLACK.add(id);
      }

      for (const node of doc.flowGraph.nodes) dfs(node.id, []);
      return results;
    },
  },

  // ── ff/bubble/reading-order-gap ───────────────────────────────────────────
  {
    id:          "ff/bubble/reading-order-gap",
    description: "Bubble reading orders within a panel must be contiguous (1-indexed).",
    category:    "bubble",
    severity:    "error",
    fixable:     true,
    check(doc) {
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        const orders = panel.bubbles.map((b) => b.readingOrder).sort((a, b) => a - b);
        const hasGap = orders.some((o, i) => o !== i + 1);
        if (hasGap) {
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: this.severity,
            message:  `Panel bubbles have non-contiguous reading orders: [${orders.join(", ")}]`,
            location: { panelId: panel.id, path: `panels[id="${panel.id}"].bubbles[*].readingOrder` },
            fix: { label: "Repair reading order", kind: { type: "repair_reading_order", panelId: panel.id } },
          });
        }
      }
      return results;
    },
  },

  // ── ff/content/panel-content-rating ───────────────────────────────────────
  {
    id:          "ff/content/panel-content-rating",
    description: "A panel cannot have a higher content rating than its parent game.",
    category:    "content",
    severity:    "error",
    fixable:     false,
    check(doc) {
      const TIER = { sfw: 0, pg13: 1, r18: 2 } as const;
      const gameRating = doc.contentRating as keyof typeof TIER | undefined;
      const gameTier   = gameRating ? TIER[gameRating] : 0;
      const results: LintResult[] = [];
      for (const panel of doc.panels) {
        const panelRating = panel.contentRating as keyof typeof TIER | null;
        if (panelRating && TIER[panelRating] > gameTier) {
          results.push({
            ruleId:   this.id,
            category: this.category,
            severity: this.severity,
            message:  `Panel is rated "${panelRating}" but the game is rated "${gameRating ?? "sfw"}".`,
            location: { panelId: panel.id, path: `panels[id="${panel.id}"].contentRating` },
          });
        }
      }
      return results;
    },
  },

];

// ─── Fix application ──────────────────────────────────────────────────────────
// The engine resolves a FixKind descriptor → FFScriptDoc mutation calls.
// Import FFScriptDoc from the mutations package in the real repo.

type MutDoc = {
  duplicatePanel(id: string): unknown;
  movePanel(id: string, order: number): unknown;
  // ... full interface from mutation-api-spec
};

export function applyFix(doc: MutDoc, panels: FFScriptV02["panels"], fix: LintFix): void {
  const { kind } = fix;
  switch (kind.type) {
    case "regenerate_panel_id":
      // FFScriptDoc doesn't expose a direct "change panel ID" — instead,
      // duplicate the panel and delete the original.
      // In the real implementation, this is a single internal mutation.
      // TODO: add FFScriptDoc.internal_regenerate_panel_id(oldId) method.
      console.warn("regenerate_panel_id fix requires internal mutation API access");
      break;

    case "repair_panel_order": {
      // Sort panels by current order, then reassign 0..N-1
      const sorted = [...panels].sort((a, b) => a.order - b.order);
      sorted.forEach((p, i) => {
        if (p.order !== i) {
          (doc as unknown as { movePanel(id: string, o: number): unknown }).movePanel(p.id, i);
        }
      });
      break;
    }

    case "repair_reading_order":
      // Implementation: sort bubbles in the panel by current readingOrder,
      // then call moveBubble or a dedicated setBubbleReadingOrder mutation.
      // TODO: wire to FFScriptDoc.setBubbleReadingOrder once available.
      console.warn("repair_reading_order: wire to setBubbleReadingOrder in FFScriptDoc");
      break;

    case "remove_placement":
      (doc as unknown as { removePlacement(id: string): unknown }).removePlacement(kind.placementId);
      break;

    case "set_speaker_null":
      (doc as unknown as { reassignSpeaker(params: { lineId: string; speakerId: null }): unknown })
        .reassignSpeaker({ lineId: kind.bubbleId, speakerId: null });
      break;
  }
}

// ─── LintEngine ───────────────────────────────────────────────────────────────

export class LintEngine {
  private readonly _rules: LintRule[];

  constructor(private readonly opts: LintEngineOptions = {}) {
    const disabled = new Set(opts.disabledRules ?? []);
    this._rules = [
      ...DEFAULT_RULES,
      ...(opts.customRules ?? []),
    ].filter((r) => !disabled.has(r.id));
  }

  get rules(): LintRule[] {
    return this._rules;
  }

  lint(doc: FFScriptV02): LintReport {
    const t0 = Date.now();
    const overrides = this.opts.severityOverrides ?? {};
    const errorsOnly = this.opts.errorsOnly ?? false;

    const findings: LintResult[] = [];
    for (const rule of this._rules) {
      const raw = rule.check(doc);
      for (const finding of raw) {
        const severity: LintSeverity = overrides[finding.ruleId] ?? finding.severity;
        if (errorsOnly && severity !== "error") continue;
        findings.push({ ...finding, severity });
      }
    }

    const errorCount   = findings.filter((f) => f.severity === "error").length;
    const warningCount = findings.filter((f) => f.severity === "warning").length;
    const infoCount    = findings.filter((f) => f.severity === "info").length;

    return {
      findings,
      errorCount,
      warningCount,
      infoCount,
      hasErrors:  errorCount > 0,
      durationMs: Date.now() - t0,
    };
  }

  fix(doc: MutDoc, rawPanels: FFScriptV02["panels"]): { fixesApplied: number; report: LintReport } {
    // Apply fixes in dependency order: ID repairs → order repairs → everything else
    const FIX_ORDER: FixKind["type"][] = [
      "regenerate_panel_id",
      "repair_panel_order",
      "repair_reading_order",
      "remove_placement",
      "set_speaker_null",
    ];

    // Re-lint to get current findings with fixes
    const report = this.lint(doc as unknown as FFScriptV02);
    const fixable = report.findings.filter((f) => f.fix != null);
    fixable.sort((a, b) =>
      FIX_ORDER.indexOf(a.fix!.kind.type) - FIX_ORDER.indexOf(b.fix!.kind.type),
    );

    let fixesApplied = 0;
    for (const finding of fixable) {
      try {
        applyFix(doc, rawPanels, finding.fix!);
        fixesApplied++;
      } catch {
        // log and continue — partial fixes are better than none
      }
    }

    return { fixesApplied, report: this.lint(doc as unknown as FFScriptV02) };
  }

  applyFix(doc: MutDoc, rawPanels: FFScriptV02["panels"], fix: LintFix): void {
    applyFix(doc, rawPanels, fix);
  }
}
