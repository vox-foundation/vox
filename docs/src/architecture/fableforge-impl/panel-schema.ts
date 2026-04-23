/**
 * FFScript Panel Schema — v0.2.0
 *
 * Drop this file into: src/lib/ffscript/panel-schema.ts
 *
 * Implements: T-021, T-022, T-025, T-031, T-035
 * Spec:       docs/src/architecture/ffscript-panel-schema-spec-2026.md
 *
 * IMPORTANT before merging:
 *   1. Verify the existing FFScriptV1Schema field names match what's referenced here.
 *   2. Confirm uuid() import — project may use a different UUID library.
 *   3. Run the full test suite after adding this to ensure no schema drift.
 */

import { z } from "zod";
import { v4 as uuid } from "uuid";

// ─── Branded scalars ──────────────────────────────────────────────────────────

export type PanelId     = string & { readonly _brand: "PanelId" };
export type NodeId      = string & { readonly _brand: "NodeId" };
export type LineId      = string & { readonly _brand: "LineId" };
export type SceneId     = string & { readonly _brand: "SceneId" };
export type CharacterId = string & { readonly _brand: "CharacterId" };
export type BubbleId    = string & { readonly _brand: "BubbleId" };
export type PlacementId = string & { readonly _brand: "PlacementId" };
export type EdgeId      = string & { readonly _brand: "EdgeId" };
export type DocVersion  = number & { readonly _brand: "DocVersion" };

export const newPanelId     = () => uuid() as PanelId;
export const newSceneId     = () => uuid() as SceneId;
export const newBubbleId    = () => uuid() as BubbleId;
export const newPlacementId = () => uuid() as PlacementId;
export const newNodeId      = () => uuid() as NodeId;

// ─── Aspect ratio → workflow dimensions (replaces hardcoded 1344×768 etc.) ───

export const AspectRatioSchema = z.enum([
  "16:9",
  "9:16",
  "1:1",
  "4:3",
  "3:4",
  "21:9",
  "2:3",
  "custom",
]);
export type AspectRatio = z.infer<typeof AspectRatioSchema>;

const ASPECT_DIMS: Record<Exclude<AspectRatio, "custom">, { w: number; h: number }> = {
  "16:9": { w: 1344, h: 768  },
  "9:16": { w: 768,  h: 1344 },
  "1:1":  { w: 1024, h: 1024 },
  "4:3":  { w: 1152, h: 896  },
  "3:4":  { w: 896,  h: 1152 },
  "21:9": { w: 1536, h: 640  },
  "2:3":  { w: 832,  h: 1248 },
};

export function aspectRatioDims(
  r: AspectRatio,
  customW?: number,
  customH?: number,
): { w: number; h: number } {
  if (r === "custom") {
    const clamp = (v: number) => Math.min(2048, Math.max(512, Math.round(v)));
    return { w: clamp(customW ?? 1024), h: clamp(customH ?? 1024) };
  }
  return ASPECT_DIMS[r];
}

// ─── Build type — distinguishes authoring placeholder from publishable ────────

export const BuildTypeSchema = z.enum(["placeholder", "draft", "published"]);
export type BuildType = z.infer<typeof BuildTypeSchema>;

// ─── Content rating ───────────────────────────────────────────────────────────

export const ContentRatingSchema = z.enum(["sfw", "pg13", "r18"]);
export type ContentRating = z.infer<typeof ContentRatingSchema>;

// ─── Background ───────────────────────────────────────────────────────────────

export const BackgroundSchema = z.object({
  assetKey:     z.string().min(1),
  prompt:       z.string().default(""),
  modelId:      z.string().min(1),
  seed:         z.number().int().nullable().default(null),
  seedLocked:   z.boolean().default(false),
  maskCacheKey: z.string().nullable().default(null),
});
export type Background = z.infer<typeof BackgroundSchema>;

// ─── Character placement ──────────────────────────────────────────────────────

export const CharacterPlacementSchema = z.object({
  id:          z.string().uuid(),
  characterId: z.string().uuid(),
  pose:        z.string().default("neutral"),
  x:           z.number(),
  y:           z.number(),
  scale:       z.number().positive().default(1.0),
  flipX:       z.boolean().default(false),
  zIndex:      z.number().int().default(0),
});
export type CharacterPlacement = z.infer<typeof CharacterPlacementSchema>;

// ─── Bubble ───────────────────────────────────────────────────────────────────

export const BubbleStyleSchema = z.enum([
  "round",
  "square",
  "thought",
  "shout",
  "whisper",
  "caption",
]);
export type BubbleStyle = z.infer<typeof BubbleStyleSchema>;

export const BubbleSchema = z.object({
  id:           z.string().uuid(),
  lineId:       z.string().uuid(),
  x:            z.number(),
  y:            z.number(),
  w:            z.number().positive(),
  h:            z.number().positive(),
  tailX:        z.number().nullable(),
  tailY:        z.number().nullable(),
  style:        BubbleStyleSchema.default("round"),
  readingOrder: z.number().int().positive(),
  fontPreset:   z.string().nullable().default(null),
}).refine(
  (b) => b.style === "caption" || (b.tailX !== null && b.tailY !== null),
  { message: "Non-caption bubbles must have tailX and tailY." },
);
export type Bubble = z.infer<typeof BubbleSchema>;

// ─── Caption ──────────────────────────────────────────────────────────────────

export const CaptionSchema = z.object({
  text:       z.string(),
  position:   z.enum(["top", "bottom", "floating"]).default("top"),
  fontPreset: z.string().nullable().default(null),
});
export type Caption = z.infer<typeof CaptionSchema>;

// ─── Panel ────────────────────────────────────────────────────────────────────

export const PanelModeSchema = z.enum(["vn", "comic", "cinematic"]);
export type PanelMode = z.infer<typeof PanelModeSchema>;

export const TransitionSchema = z.enum(["cut", "fade", "wipe", "iris"]);
export type Transition = z.infer<typeof TransitionSchema>;

export const PanelMetadataSchema = z.object({
  lockedSeed:       z.number().int().nullable().default(null),
  artDirectionTags: z.array(z.string()).default([]),
  locked:           z.boolean().default(false),
  seedHistory:      z.array(z.number().int()).max(10).default([]),
});

export const PanelSchema = z.object({
  id:              z.string().uuid(),
  order:           z.number().int().nonneg(),
  beatId:          z.string().uuid().nullable().default(null),
  dialogueNodeIds: z.array(z.string().uuid()).default([]),
  sceneElementIds: z.array(z.string().uuid()).default([]),
  bounds: z.object({
    x: z.number().nonneg(),
    y: z.number().nonneg(),
    w: z.number().positive(),
    h: z.number().positive(),
  }).nullable().default(null),
  aspectRatio:   AspectRatioSchema.default("16:9"),
  customDims:    z.object({ w: z.number().int(), h: z.number().int() }).nullable().default(null),
  background:    BackgroundSchema.nullable().default(null),
  foreground:    z.object({
    characterPlacements: z.array(CharacterPlacementSchema).default([]),
  }).default({ characterPlacements: [] }),
  bubbles:       z.array(BubbleSchema).default([]),
  caption:       CaptionSchema.nullable().default(null),
  transition:    TransitionSchema.default("cut"),
  mode:          PanelModeSchema.default("vn"),
  contentRating: ContentRatingSchema.nullable().default(null),
  metadata:      PanelMetadataSchema.default({}),
});
export type Panel = z.infer<typeof PanelSchema>;

// ─── Scene ────────────────────────────────────────────────────────────────────

export const SceneSchema = z.object({
  id:                     z.string().uuid(),
  title:                  z.string().default("Scene"),
  panelIds:               z.array(z.string().uuid()).default([]),
  backgroundMusicThemeId: z.string().nullable().default(null),
});
export type Scene = z.infer<typeof SceneSchema>;

// ─── FFScript v0.2 extension ──────────────────────────────────────────────────
//
// In src/lib/ffscript/schema.ts, extend the existing FFScriptV1Schema:
//
//   export const FFScriptSchema = FFScriptV1Schema.extend({
//     version:   z.literal("0.2.0"),
//     buildType: BuildTypeSchema.default("draft"),
//     panels:    z.array(PanelSchema).default([]),
//     scenes:    z.array(SceneSchema).default([]),
//   });
//
// Also update the version regex from /^0\.\d+\.\d+$/ to /^\d+\.\d+\.\d+$/
// so future 1.x.x versions validate without another regex change.

// ─── Migration: v0.1.0 → v0.2.0 ──────────────────────────────────────────────

/**
 * Run once when opening a v0.1.0 document.
 * Creates one Panel per DialogueNode/ChoiceNode, grouped in a single "Act 1" scene.
 * Panels start with no background — linter rule ff/background/missing-background fires,
 * driving the user to generate images.
 *
 * IMPORTANT: Verify the shape of your FlowGraph nodes before deploying.
 * The field names below (node.type, node.id) must match your actual schema.
 */
export function migrateV01toV02(doc: Record<string, unknown>): Record<string, unknown> {
  // Guard: already migrated
  if ((doc as { version?: string }).version === "0.2.0") return doc;

  const flowGraph = (doc as { flowGraph?: { nodes?: Array<{ type: string; id: string }> } })
    .flowGraph ?? { nodes: [] };

  const eligibleNodes = (flowGraph.nodes ?? []).filter(
    (n) => n.type === "dialogue" || n.type === "choice",
  );

  const panels: Panel[] = eligibleNodes.map((node, idx) =>
    PanelSchema.parse({
      id:              newPanelId(),
      order:           idx,
      beatId:          null,
      dialogueNodeIds: [node.id],
      sceneElementIds: [],
    }),
  );

  const scene: Scene = SceneSchema.parse({
    id:       newSceneId(),
    title:    "Act 1",
    panelIds: panels.map((p) => p.id),
  });

  return {
    ...doc,
    version:   "0.2.0",
    buildType: "draft" as BuildType,
    panels,
    scenes: [scene],
  };
}

// ─── Post-mutation invariant checker (called by mutation API after every write) ─

export type InvariantViolation = {
  code:    string;
  message: string;
  panelId?: string;
};

export function checkPanelInvariants(
  panels: Panel[],
  scenes: Scene[],
  characters: Array<{ id: string }>,
): InvariantViolation[] {
  const violations: InvariantViolation[] = [];

  // 1. Order must be 0-indexed contiguous with no gaps or duplicates
  const sortedOrders = [...panels].map((p) => p.order).sort((a, b) => a - b);
  sortedOrders.forEach((order, idx) => {
    if (order !== idx) {
      violations.push({
        code:    "PANEL_ORDER_GAP",
        message: `Panel order is not contiguous: expected ${idx}, found ${order}`,
      });
    }
  });

  // 2. Duplicate panel IDs
  const seenIds = new Set<string>();
  for (const panel of panels) {
    if (seenIds.has(panel.id)) {
      violations.push({
        code:    "DUPLICATE_PANEL_ID",
        message: `Duplicate panel id: ${panel.id}`,
        panelId: panel.id,
      });
    }
    seenIds.add(panel.id);
  }

  // 3. Bubble readingOrder contiguity per panel
  for (const panel of panels) {
    const orders = panel.bubbles.map((b) => b.readingOrder).sort((a, b) => a - b);
    orders.forEach((o, idx) => {
      if (o !== idx + 1) {
        violations.push({
          code:    "BUBBLE_READING_ORDER_GAP",
          message: `Panel ${panel.id}: bubble reading orders are not contiguous`,
          panelId: panel.id,
        });
      }
    });
  }

  // 4. Panel contentRating ≤ game contentRating is checked at the doc level,
  //    not here, because we don't have the game rating in scope.

  return violations;
}
