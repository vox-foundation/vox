/**
 * Beat → Panel Compiler (T-023)
 *
 * Drop into: src/lib/studio/compiler.ts  (replacing compileBeatDraftsToFFScript)
 *
 * Converts gameDrafts.beats (the wizard's intermediate format) into a fully
 * formed FFScript v0.2 document with panels, scenes, flow-graph nodes, and
 * character placements.
 *
 * IMPORTANT before merging:
 *   1. Verify the Beat and GameDraft types match your actual Convex schema.
 *   2. The character anchor lookup (step 4) depends on characterAnchors table.
 *   3. 1:1 beat-per-panel is the default (audit §8 Q1 decision).
 *      Multi-panel beats are allowed via beat.panelCount > 1 (optional field).
 */

import { v4 as uuid } from "uuid";
import {
  PanelSchema, SceneSchema,
  type Panel, type Scene, type AspectRatio,
} from "./panel-schema";

// ─── Wizard types (adapt to match your actual schema) ────────────────────────

interface Beat {
  id:                  string;
  order:               number;
  /** LLM-generated scene description / prompt for image generation. */
  visualPrompt:        string;
  /** Dialogue lines in this beat. */
  lines:               Array<{
    id:       string;
    speakerId: string | null;
    text:      string;
  }>;
  /** IDs of characters present in this beat. */
  characterIds:        string[];
  /** R2 key of the already-generated background image (may be null if not yet generated). */
  generatedAssetKey:   string | null;
  /** Model used to generate the asset. */
  generatedModelId:    string | null;
  /** Seed used. */
  generatedSeed:       number | null;
  /** Optional: allow a beat to span multiple panels (advanced mode). Default 1. */
  panelCount?:         number;
  /** Content rating for this beat. null = inherit from game. */
  contentRating?:      "sfw" | "pg13" | "r18" | null;
}

interface Character {
  id:        string;
  name:      string;
  aliasIds?: string[];
}

interface GameDraft {
  id:            string;
  title:         string;
  beats:         Beat[];
  characters:    Character[];
  contentRating: "sfw" | "pg13" | "r18";
  /** User's preferred aspect ratio. */
  aspectRatio?:  AspectRatio;
  /** Scene grouping: beats in the same scene share a music theme. */
  sceneBreaks?:  number[]; // beat indices that start a new scene
}

// ─── Compiler output ──────────────────────────────────────────────────────────

interface FFScriptV02 {
  version:       string;
  buildType:     string;
  panels:        Panel[];
  scenes:        Scene[];
  characters:    Array<{ id: string; name: string }>;
  contentRating: string;
  flowGraph: {
    nodes: Array<{ id: string; type: string; speakerId?: string | null; text?: string }>;
    edges: Array<{ id: string; from: string; to: string }>;
    variables: Array<{ name: string }>;
  };
  metadata: {
    draftId:    string;
    compiledAt: string;
    title:      string;
  };
}

// ─── compileBeatDraftsToFFScript ──────────────────────────────────────────────

/**
 * Main entry point.
 *
 * 1. Build a flowGraph node per dialogue line in each beat.
 * 2. Build one Panel per beat (or beat.panelCount panels for multi-panel beats).
 * 3. Wire the panel to its flow nodes.
 * 4. Place characters in the foreground using simple positioning heuristics.
 * 5. Group panels into scenes based on draft.sceneBreaks.
 * 6. Return a fully valid FFScript v0.2 document.
 */
export function compileBeatDraftsToFFScript(draft: GameDraft): FFScriptV02 {
  const aspectRatio = draft.aspectRatio ?? "16:9";
  const nodes:       FFScriptV02["flowGraph"]["nodes"] = [];
  const edges:       FFScriptV02["flowGraph"]["edges"] = [];
  const panels:      Panel[] = [];

  // ── Start node ──────────────────────────────────────────────────────────
  const startNodeId = uuid();
  nodes.push({ id: startNodeId, type: "start" });

  let prevNodeId = startNodeId;
  let globalOrder = 0;

  for (const beat of [...draft.beats].sort((a, b) => a.order - b.order)) {
    const panelCount = Math.max(1, beat.panelCount ?? 1);

    // For multi-panel beats, distribute lines evenly across panels
    const lineChunks = chunkArray(beat.lines, panelCount);

    for (let pi = 0; pi < panelCount; pi++) {
      const linesForPanel = lineChunks[pi] ?? [];
      const dialogueNodeIds: string[] = [];

      // ── Flow nodes for each line in this panel ────────────────────────
      for (const line of linesForPanel) {
        const nodeId = uuid();
        nodes.push({
          id:        nodeId,
          type:      "dialogue",
          speakerId: line.speakerId,
          text:      line.text,
        });
        dialogueNodeIds.push(nodeId);

        // Chain: prevNode → this node
        edges.push({ id: uuid(), from: prevNodeId, to: nodeId });
        prevNodeId = nodeId;
      }

      // ── Bubbles for each line ──────────────────────────────────────────
      const bubbles = linesForPanel.map((line, li) => ({
        id:           uuid(),
        lineId:       dialogueNodeIds[li]!,
        x:            0.05,
        y:            0.05 + li * 0.22,
        w:            0.45,
        h:            0.18,
        tailX:        line.speakerId ? 0.25 : null,
        tailY:        line.speakerId ? 0.85 : null,
        style:        (line.speakerId ? "round" : "caption") as "round" | "caption",
        readingOrder: li + 1,
        fontPreset:   null,
      }));

      // ── Character placements (heuristic: spread across thirds) ─────────
      const presentChars = draft.characters.filter(
        (c) => beat.characterIds.includes(c.id),
      );
      const characterPlacements = presentChars.map((char, ci) => ({
        id:          uuid(),
        characterId: char.id,
        pose:        "neutral",
        x:           characterX(ci, presentChars.length),
        y:           0.2,
        scale:       0.8,
        flipX:       ci % 2 === 1, // alternate facing direction
        zIndex:      ci,
      }));

      // ── Background ────────────────────────────────────────────────────
      // Only the first panel of a beat carries the generated asset.
      const hasAsset = pi === 0 && beat.generatedAssetKey;
      const background = hasAsset
        ? {
            assetKey:     beat.generatedAssetKey!,
            prompt:       beat.visualPrompt,
            modelId:      beat.generatedModelId ?? "unknown",
            seed:         beat.generatedSeed    ?? null,
            seedLocked:   false,
            maskCacheKey: null,
          }
        : null;

      const panel = PanelSchema.parse({
        id:              uuid(),
        order:           globalOrder++,
        beatId:          beat.id,
        dialogueNodeIds,
        sceneElementIds: [],
        aspectRatio,
        background,
        foreground: { characterPlacements },
        bubbles,
        transition:    "cut",
        mode:          "vn",
        contentRating: beat.contentRating ?? null,
        metadata: {
          lockedSeed:       beat.generatedSeed ?? null,
          artDirectionTags: [],
          locked:           false,
          seedHistory:      beat.generatedSeed != null ? [beat.generatedSeed] : [],
        },
      });

      panels.push(panel);
    }
  }

  // ── End node ─────────────────────────────────────────────────────────────
  const endNodeId = uuid();
  nodes.push({ id: endNodeId, type: "end" });
  edges.push({ id: uuid(), from: prevNodeId, to: endNodeId });

  // ── Scenes (T-033) ────────────────────────────────────────────────────────
  const scenes = buildScenes(panels, draft.sceneBreaks ?? []);

  return {
    version:       "0.2.0",
    buildType:     "draft",
    panels,
    scenes,
    characters:    draft.characters.map(({ id, name }) => ({ id, name })),
    contentRating: draft.contentRating,
    flowGraph:     { nodes, edges, variables: [] },
    metadata: {
      draftId:    draft.id,
      compiledAt: new Date().toISOString(),
      title:      draft.title,
    },
  };
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/** Simple heuristic: spread N characters across the horizontal thirds. */
function characterX(index: number, total: number): number {
  if (total === 1) return 0.35;
  if (total === 2) return index === 0 ? 0.05 : 0.55;
  // 3+: divide into equal slots
  return 0.05 + (index / total) * 0.9;
}

/** Chunk an array into N roughly-equal parts. */
function chunkArray<T>(arr: T[], n: number): T[][] {
  const size   = Math.ceil(arr.length / n);
  const chunks: T[][] = [];
  for (let i = 0; i < n; i++) {
    chunks.push(arr.slice(i * size, (i + 1) * size));
  }
  return chunks;
}

/**
 * Group panels into scenes.
 * sceneBreaks: array of beat-order indices that start a new scene.
 * If empty, all panels go into a single "Act 1" scene.
 */
function buildScenes(panels: Panel[], sceneBreaks: number[]): Scene[] {
  if (sceneBreaks.length === 0) {
    return [SceneSchema.parse({
      id:       uuid(),
      title:    "Act 1",
      panelIds: panels.map((p) => p.id),
    })];
  }

  const breakSet = new Set(sceneBreaks);
  const scenes: Scene[] = [];
  let   current: string[] = [];
  let   sceneNum = 1;

  for (const panel of panels.sort((a, b) => a.order - b.order)) {
    // A beat panel at a scene-break index starts a new scene
    if (breakSet.has(panel.order) && current.length > 0) {
      scenes.push(SceneSchema.parse({
        id:       uuid(),
        title:    `Act ${sceneNum++}`,
        panelIds: current,
      }));
      current = [];
    }
    current.push(panel.id);
  }

  if (current.length > 0) {
    scenes.push(SceneSchema.parse({
      id:       uuid(),
      title:    `Act ${sceneNum}`,
      panelIds: current,
    }));
  }

  return scenes;
}

// ─── Reverse compiler: FFScript → draft beats (for T-105 "Continue with AI") ──

/**
 * Reconstitute a minimal GameDraft from an existing FFScript document.
 * Used when the user opens an already-edited game in the wizard ("Continue with AI").
 * Only recovers fields that can be reliably inferred; manual edits to bubbles or
 * character placements are lost (they must be re-specified in the wizard).
 */
export function decompileFFScriptToDraft(doc: FFScriptV02): Omit<GameDraft, "id"> {
  const beats: Beat[] = doc.panels
    .sort((a, b) => a.order - b.order)
    .map((panel) => {
      const lines = panel.dialogueNodeIds
        .map((nid) => doc.flowGraph.nodes.find((n) => n.id === nid))
        .filter(Boolean)
        .map((node) => ({
          id:        node!.id,
          speakerId: node!.speakerId ?? null,
          text:      node!.text ?? "",
        }));

      return {
        id:                panel.id,
        order:             panel.order,
        visualPrompt:      panel.background?.prompt ?? "",
        lines,
        characterIds:      panel.foreground.characterPlacements.map((cp) => cp.characterId),
        generatedAssetKey: panel.background?.assetKey ?? null,
        generatedModelId:  panel.background?.modelId  ?? null,
        generatedSeed:     panel.background?.seed      ?? null,
        contentRating:     panel.contentRating,
      };
    });

  return {
    title:         doc.metadata.title,
    beats,
    characters:    doc.characters,
    contentRating: doc.contentRating as "sfw" | "pg13" | "r18",
    aspectRatio:   doc.panels[0]?.aspectRatio,
    sceneBreaks:   [], // scenes must be re-specified manually
  };
}
