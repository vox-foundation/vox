/**
 * FFScriptDoc — Immer-based mutation wrapper
 *
 * Drop into: packages/ffscript/src/mutations/doc.ts
 *
 * Implements: T-041, T-042, T-043, T-044, T-045, T-051, T-054
 * Spec:       docs/src/architecture/ffscript-mutation-api-spec-2026.md
 *
 * Dependencies (install in packages/ffscript):
 *   pnpm add immer fast-json-patch uuid zod
 *
 * IMPORTANT before merging:
 *   1. Replace the FFScriptV02 import with your real schema types.
 *   2. The forgivingValidator (T-051) pre-processes LLM blobs at open() time only.
 *   3. UNDO_LIMIT is tunable — default 100.
 */

import { produce, enableMapSet } from "immer";
import * as jsonpatch from "fast-json-patch";
import { v4 as uuid } from "uuid";
import { PanelSchema, SceneSchema, BubbleSchema, CharacterPlacementSchema } from "./panel-schema";
import { checkPanelInvariants } from "./panel-schema";
import type {
  Panel, Scene, Bubble, CharacterPlacement,
  PanelId, NodeId, LineId, SceneId, CharacterId,
  BubbleId, PlacementId, EdgeId, AspectRatio, PanelMode, Transition,
} from "./panel-schema";

enableMapSet();

// ─── Version counter ──────────────────────────────────────────────────────────

export type DocVersion = number & { readonly _brand: "DocVersion" };
let _nextVersion = 1 as DocVersion;
const nextVersion = (): DocVersion => _nextVersion++ as DocVersion;

// ─── Errors ───────────────────────────────────────────────────────────────────

export type MutationErrorCode =
  | "VALIDATION_FAILED"
  | "NOT_FOUND"
  | "DUPLICATE_ID"
  | "INVARIANT_VIOLATION"
  | "VERSION_CONFLICT"
  | "UNDO_STACK_EMPTY"
  | "REDO_STACK_EMPTY"
  | "READ_ONLY";

export class FFScriptMutationError extends Error {
  constructor(
    public readonly code: MutationErrorCode,
    message: string,
    public readonly context?: Record<string, unknown>,
  ) {
    super(message);
    this.name = "FFScriptMutationError";
  }
}

// ─── Patch types ──────────────────────────────────────────────────────────────

export interface MutationRecord {
  label:       string;
  forward:     jsonpatch.Operation[];
  reverse:     jsonpatch.Operation[];
  fromVersion: DocVersion;
  toVersion:   DocVersion;
  timestamp:   number;
}

export interface MutationResult<T = void> {
  value:      T;
  diff:       MutationRecord;
  newVersion: DocVersion;
}

// Minimal FFScript v0.2 shape — replace with real import from your schema
interface FFScriptV02 {
  version:     string;
  buildType?:  string;
  panels:      Panel[];
  scenes:      Scene[];
  characters:  Array<{ id: string; name: string }>;
  flowGraph: {
    nodes:     Array<{ id: string; type: string; speakerId?: string | null; text?: string }>;
    edges:     Array<{ id: string; from: string; to: string }>;
    variables?: Array<{ name: string }>;
  };
  contentRating?: string;
}

const UNDO_LIMIT = 100;

// ─── FFScriptDoc ──────────────────────────────────────────────────────────────

export class FFScriptDoc {
  private _state:   FFScriptV02;
  private _version: DocVersion;
  private readonly _readOnly: boolean;
  private _past:    MutationRecord[] = [];
  private _future:  MutationRecord[] = [];

  private constructor(state: FFScriptV02, readOnly = false) {
    this._state   = state;
    this._version = nextVersion();
    this._readOnly = readOnly;
  }

  // ── Construction ─────────────────────────────────────────────────────────

  static open(raw: FFScriptV02, opts?: { readOnly?: boolean }): FFScriptDoc {
    // T-051: forgiving pre-process at open time (repair LLM blobs before strict validate)
    const forgiven = FFScriptDoc._forgivingRepair(raw);
    FFScriptDoc._strictValidate(forgiven);
    return new FFScriptDoc(forgiven, opts?.readOnly ?? false);
  }

  serialize(): string {
    return JSON.stringify(this._state, Object.keys(this._state).sort());
  }

  get version(): DocVersion { return this._version; }
  get state():   FFScriptV02  { return this._state;   } // read-only access for linter

  // ── Panel mutations ───────────────────────────────────────────────────────

  insertPanel(params: {
    afterId?:     PanelId | null;
    mode?:        PanelMode;
    aspectRatio?: AspectRatio;
    sceneId?:     SceneId;
  }): MutationResult<PanelId> {
    this._guardWrite();
    const newId = uuid() as PanelId;

    return this._apply(`Insert panel`, (draft) => {
      // Determine insertion order
      let insertOrder = 0;
      if (params.afterId) {
        const after = draft.panels.find((p) => p.id === params.afterId);
        if (!after) throw new FFScriptMutationError("NOT_FOUND", `Panel "${params.afterId}" not found`);
        insertOrder = after.order + 1;
      }

      // Shift subsequent panels
      draft.panels.forEach((p) => { if (p.order >= insertOrder) p.order++; });

      // Create the new panel
      const newPanel = PanelSchema.parse({
        id:          newId,
        order:       insertOrder,
        mode:        params.mode       ?? "vn",
        aspectRatio: params.aspectRatio ?? "16:9",
      });
      draft.panels.push(newPanel);

      // Add to scene
      const targetSceneId = params.sceneId ?? (
        params.afterId
          ? draft.scenes.find((s) => s.panelIds.includes(params.afterId!))?.id
          : draft.scenes[0]?.id
      );
      if (targetSceneId) {
        const scene = draft.scenes.find((s) => s.id === targetSceneId);
        if (scene) {
          const afterIdx = params.afterId
            ? scene.panelIds.indexOf(params.afterId)
            : scene.panelIds.length - 1;
          scene.panelIds.splice(afterIdx + 1, 0, newId);
        }
      }
    }, newId);
  }

  deletePanel(id: PanelId): MutationResult<{
    removedNodeIds:     NodeId[];
    removedEdgeIds:     EdgeId[];
    removedBubbleIds:   BubbleId[];
    removedPlacementIds: PlacementId[];
  }> {
    this._guardWrite();
    const panel = this._state.panels.find((p) => p.id === id);
    if (!panel) throw new FFScriptMutationError("NOT_FOUND", `Panel "${id}" not found`);

    const removedBubbleIds    = panel.bubbles.map((b) => b.id as BubbleId);
    const removedPlacementIds = panel.foreground.characterPlacements.map((p) => p.id as PlacementId);
    const removedNodeIds      = panel.dialogueNodeIds as NodeId[];
    const removedEdgeIds: EdgeId[] = [];

    return this._apply(`Delete panel`, (draft) => {
      // Remove edges targeting this panel's nodes
      const nodeSet = new Set(panel.dialogueNodeIds);
      draft.flowGraph.edges = draft.flowGraph.edges.filter((e) => {
        const remove = nodeSet.has(e.to) || nodeSet.has(e.from);
        if (remove) removedEdgeIds.push(e.id as EdgeId);
        return !remove;
      });

      // Remove panel from scenes
      draft.scenes.forEach((s) => {
        s.panelIds = s.panelIds.filter((pid) => pid !== id);
      });

      // Compact order
      draft.panels = draft.panels
        .filter((p) => p.id !== id)
        .sort((a, b) => a.order - b.order)
        .map((p, i) => ({ ...p, order: i }));
    }, { removedNodeIds, removedEdgeIds, removedBubbleIds, removedPlacementIds });
  }

  movePanel(id: PanelId, toOrder: number): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Move panel to position ${toOrder}`, (draft) => {
      const panel = draft.panels.find((p) => p.id === id);
      if (!panel) throw new FFScriptMutationError("NOT_FOUND", `Panel "${id}" not found`);

      const sorted   = [...draft.panels].sort((a, b) => a.order - b.order);
      const from     = panel.order;
      const to       = Math.max(0, Math.min(toOrder, sorted.length - 1));
      if (from === to) return;

      // Rotate the order array
      sorted.splice(from, 1);
      sorted.splice(to, 0, panel);
      sorted.forEach((p, i) => {
        const live = draft.panels.find((x) => x.id === p.id)!;
        live.order = i;
      });
    }, undefined);
  }

  duplicatePanel(id: PanelId): MutationResult<PanelId> {
    this._guardWrite();
    const source = this._state.panels.find((p) => p.id === id);
    if (!source) throw new FFScriptMutationError("NOT_FOUND", `Panel "${id}" not found`);
    const newId = uuid() as PanelId;

    return this._apply(`Duplicate panel`, (draft) => {
      const insertOrder = source.order + 1;
      draft.panels.forEach((p) => { if (p.order >= insertOrder) p.order++; });

      const clone: Panel = {
        ...JSON.parse(JSON.stringify(source)), // deep clone
        id:      newId,
        order:   insertOrder,
        bubbles: source.bubbles.map((b) => ({ ...b, id: uuid() })),
        foreground: {
          characterPlacements: source.foreground.characterPlacements.map(
            (cp) => ({ ...cp, id: uuid() }),
          ),
        },
        dialogueNodeIds: [], // clone doesn't inherit flow nodes
        metadata: { ...source.metadata, locked: false },
      };
      draft.panels.push(clone);

      // Insert into same scene
      const scene = draft.scenes.find((s) => s.panelIds.includes(id));
      if (scene) {
        const idx = scene.panelIds.indexOf(id);
        scene.panelIds.splice(idx + 1, 0, newId);
      }
    }, newId);
  }

  updatePanel(id: PanelId, patch: Partial<Pick<Panel,
    "background" | "foreground" | "mode" | "aspectRatio" | "customDims" |
    "transition" | "contentRating" | "caption" | "bounds" | "metadata"
  >>): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Update panel`, (draft) => {
      const panel = draft.panels.find((p) => p.id === id);
      if (!panel) throw new FFScriptMutationError("NOT_FOUND", `Panel "${id}" not found`);
      Object.assign(panel, patch);
    }, undefined);
  }

  // ── Dialogue mutations ────────────────────────────────────────────────────

  insertDialogueLine(params: {
    panelId:      PanelId;
    speakerId:    CharacterId | null;
    text:         string;
    afterLineId?: LineId;
  }): MutationResult<{ nodeId: NodeId; bubbleId: BubbleId }> {
    this._guardWrite();
    const nodeId   = uuid() as NodeId;
    const bubbleId = uuid() as BubbleId;

    return this._apply(`Insert dialogue line`, (draft) => {
      const panel = draft.panels.find((p) => p.id === params.panelId);
      if (!panel) throw new FFScriptMutationError("NOT_FOUND", `Panel "${params.panelId}" not found`);

      // Add flow node
      draft.flowGraph.nodes.push({
        id:        nodeId,
        type:      "dialogue",
        speakerId: params.speakerId,
        text:      params.text,
      });
      panel.dialogueNodeIds.push(nodeId);

      // Auto-position bubble (stacked below last bubble)
      const lastOrder = Math.max(0, ...panel.bubbles.map((b) => b.readingOrder));
      const bubble = BubbleSchema.parse({
        id:           bubbleId,
        lineId:       nodeId,
        x:            0.05,
        y:            0.05 + lastOrder * 0.2,
        w:            0.4,
        h:            0.15,
        tailX:        params.speakerId ? 0.3 : null,
        tailY:        params.speakerId ? 0.85 : null,
        style:        params.speakerId ? "round" : "caption",
        readingOrder: lastOrder + 1,
      });
      panel.bubbles.push(bubble);
    }, { nodeId, bubbleId });
  }

  deleteDialogueLine(id: LineId): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Delete dialogue line`, (draft) => {
      // Remove from flow graph
      draft.flowGraph.nodes = draft.flowGraph.nodes.filter((n) => n.id !== id);

      // Remove bubble + repair reading order
      for (const panel of draft.panels) {
        const before = panel.bubbles.length;
        panel.bubbles = panel.bubbles.filter((b) => b.lineId !== id);
        if (panel.bubbles.length < before) {
          panel.bubbles = panel.bubbles
            .sort((a, b) => a.readingOrder - b.readingOrder)
            .map((b, i) => ({ ...b, readingOrder: i + 1 }));
          panel.dialogueNodeIds = panel.dialogueNodeIds.filter((nid) => nid !== id);
        }
      }
    }, undefined);
  }

  reassignSpeaker(params: { lineId: LineId; speakerId: CharacterId | null }): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Reassign speaker`, (draft) => {
      const node = draft.flowGraph.nodes.find((n) => n.id === params.lineId);
      if (!node) throw new FFScriptMutationError("NOT_FOUND", `Node "${params.lineId}" not found`);
      node.speakerId = params.speakerId;

      // Update bubble style if speaker becomes null → caption
      for (const panel of draft.panels) {
        const bubble = panel.bubbles.find((b) => b.lineId === params.lineId);
        if (bubble) {
          bubble.style = params.speakerId ? "round" : "caption";
          if (!params.speakerId) { bubble.tailX = null; bubble.tailY = null; }
        }
      }
    }, undefined);
  }

  // ── Character placement ───────────────────────────────────────────────────

  placeCharacter(params: {
    panelId:     PanelId;
    characterId: CharacterId;
    pose?:       string;
    x:           number;
    y:           number;
    scale?:      number;
    flipX?:      boolean;
  }): MutationResult<PlacementId> {
    this._guardWrite();
    const placementId = uuid() as PlacementId;
    return this._apply(`Place character`, (draft) => {
      const panel = draft.panels.find((p) => p.id === params.panelId);
      if (!panel) throw new FFScriptMutationError("NOT_FOUND", `Panel "${params.panelId}" not found`);
      const maxZ = Math.max(-1, ...panel.foreground.characterPlacements.map((cp) => cp.zIndex));
      panel.foreground.characterPlacements.push(
        CharacterPlacementSchema.parse({
          id:          placementId,
          characterId: params.characterId,
          pose:        params.pose  ?? "neutral",
          x:           params.x,
          y:           params.y,
          scale:       params.scale ?? 1.0,
          flipX:       params.flipX ?? false,
          zIndex:      maxZ + 1,
        }),
      );
    }, placementId);
  }

  removePlacement(id: PlacementId): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Remove character placement`, (draft) => {
      for (const panel of draft.panels) {
        panel.foreground.characterPlacements =
          panel.foreground.characterPlacements.filter((cp) => cp.id !== id);
      }
    }, undefined);
  }

  // ── Bubble mutations ──────────────────────────────────────────────────────

  moveBubble(params: {
    panelId: PanelId; bubbleId: BubbleId;
    x?: number; y?: number; w?: number; h?: number;
    tailX?: number; tailY?: number;
  }): MutationResult<void> {
    this._guardWrite();
    return this._apply(`Move bubble`, (draft) => {
      const panel  = draft.panels.find((p) => p.id === params.panelId);
      const bubble = panel?.bubbles.find((b) => b.id === params.bubbleId);
      if (!bubble) throw new FFScriptMutationError("NOT_FOUND", `Bubble "${params.bubbleId}" not found`);
      if (params.x     !== undefined) bubble.x     = params.x;
      if (params.y     !== undefined) bubble.y     = params.y;
      if (params.w     !== undefined) bubble.w     = params.w;
      if (params.h     !== undefined) bubble.h     = params.h;
      if (params.tailX !== undefined) bubble.tailX = params.tailX;
      if (params.tailY !== undefined) bubble.tailY = params.tailY;
    }, undefined);
  }

  // ── Scene mutations ───────────────────────────────────────────────────────

  insertScene(params: { title?: string; afterSceneId?: SceneId }): MutationResult<SceneId> {
    this._guardWrite();
    const sceneId = uuid() as SceneId;
    return this._apply(`Insert scene`, (draft) => {
      const scene = SceneSchema.parse({ id: sceneId, title: params.title ?? "New Scene" });
      if (params.afterSceneId) {
        const idx = draft.scenes.findIndex((s) => s.id === params.afterSceneId);
        draft.scenes.splice(idx + 1, 0, scene);
      } else {
        draft.scenes.push(scene);
      }
    }, sceneId);
  }

  // ── Undo / redo ───────────────────────────────────────────────────────────

  get canUndo(): boolean { return this._past.length > 0; }
  get canRedo(): boolean { return this._future.length > 0; }
  get undoLabel(): string | null { return this._past.at(-1)?.label ?? null; }
  get redoLabel(): string | null { return this._future[0]?.label ?? null; }

  undo(): MutationResult<void> {
    if (!this.canUndo) throw new FFScriptMutationError("UNDO_STACK_EMPTY", "Nothing to undo");
    const record  = this._past.pop()!;
    const before  = this._state;
    this._state   = jsonpatch.applyPatch(
      JSON.parse(JSON.stringify(this._state)), record.reverse, false, false,
    ).newDocument as FFScriptV02;
    this._version = nextVersion();
    this._future.unshift(record);
    return {
      value:      undefined,
      diff:       { ...record, forward: record.reverse, reverse: record.forward },
      newVersion: this._version,
    };
  }

  redo(): MutationResult<void> {
    if (!this.canRedo) throw new FFScriptMutationError("REDO_STACK_EMPTY", "Nothing to redo");
    const record  = this._future.shift()!;
    this._state   = jsonpatch.applyPatch(
      JSON.parse(JSON.stringify(this._state)), record.forward, false, false,
    ).newDocument as FFScriptV02;
    this._version = nextVersion();
    this._past.push(record);
    if (this._past.length > UNDO_LIMIT) this._past.shift();
    return { value: undefined, diff: record, newVersion: this._version };
  }

  // ── Optimistic concurrency (T-045) ────────────────────────────────────────

  assertVersion(expected: DocVersion): void {
    if (this._version !== expected) {
      throw new FFScriptMutationError("VERSION_CONFLICT",
        `Expected version ${expected}, doc is at version ${this._version}`);
    }
  }

  // ── Batch mutations ───────────────────────────────────────────────────────

  batch<T>(label: string, fn: (doc: FFScriptDoc) => T): MutationResult<T> {
    this._guardWrite();
    const snapshot     = JSON.parse(JSON.stringify(this._state)) as FFScriptV02;
    const fromVersion  = this._version;
    let   result: T;

    try {
      result = fn(this);
    } catch (err) {
      // Roll back to snapshot
      this._state   = snapshot;
      this._version = fromVersion;
      throw err;
    }

    const forward  = jsonpatch.compare(snapshot, this._state);
    const reverse  = jsonpatch.compare(this._state, snapshot);
    // Collapse the sub-records pushed during the batch into a single record
    const collapsed = this._past.splice(
      this._past.findIndex((r) => r.fromVersion === fromVersion),
    );
    const record: MutationRecord = {
      label,
      forward:     collapsed.flatMap((r) => r.forward).concat(forward),
      reverse:     reverse,
      fromVersion,
      toVersion:   this._version,
      timestamp:   Date.now(),
    };
    this._past.push(record);
    if (this._past.length > UNDO_LIMIT) this._past.shift();
    this._future = [];

    return { value: result!, diff: record, newVersion: this._version };
  }

  // ── Private helpers ───────────────────────────────────────────────────────

  private _guardWrite(): void {
    if (this._readOnly) throw new FFScriptMutationError("READ_ONLY", "Doc is in read-only mode");
  }

  private _apply<T>(label: string, mutator: (draft: FFScriptV02) => void, value: T): MutationResult<T> {
    const before  = JSON.parse(JSON.stringify(this._state)) as FFScriptV02;
    let   after: FFScriptV02;
    try {
      after = produce(this._state, mutator);
    } catch (err) {
      if (err instanceof FFScriptMutationError) throw err;
      throw new FFScriptMutationError("INVARIANT_VIOLATION", String(err));
    }

    // Post-mutation invariant check (T-043)
    const violations = checkPanelInvariants(
      after.panels,
      after.scenes,
      after.characters,
    );
    if (violations.length > 0) {
      throw new FFScriptMutationError("VALIDATION_FAILED",
        violations.map((v) => v.message).join("; "),
        { violations });
    }

    const forward  = jsonpatch.compare(before, after);
    const reverse  = jsonpatch.compare(after, before);
    const from     = this._version;
    this._state    = after;
    this._version  = nextVersion();
    this._future   = []; // any new mutation clears redo stack

    const record: MutationRecord = { label, forward, reverse, fromVersion: from, toVersion: this._version, timestamp: Date.now() };
    this._past.push(record);
    if (this._past.length > UNDO_LIMIT) this._past.shift();

    return { value, diff: record, newVersion: this._version };
  }

  private static _forgivingRepair(raw: FFScriptV02): FFScriptV02 {
    // Fix duplicate panel IDs (common in LLM-generated docs)
    const seenIds = new Set<string>();
    const panels  = (raw.panels ?? []).map((p) => {
      if (seenIds.has(p.id)) return { ...p, id: uuid() };
      seenIds.add(p.id);
      return p;
    });

    // Fill missing required arrays
    return {
      ...raw,
      panels:  panels,
      scenes:  raw.scenes  ?? [],
      characters: raw.characters ?? [],
      flowGraph: {
        nodes:     raw.flowGraph?.nodes  ?? [],
        edges:     raw.flowGraph?.edges  ?? [],
        variables: raw.flowGraph?.variables ?? [],
      },
    };
  }

  private static _strictValidate(doc: FFScriptV02): void {
    const violations = checkPanelInvariants(doc.panels, doc.scenes, doc.characters);
    if (violations.length > 0) {
      throw new FFScriptMutationError("VALIDATION_FAILED",
        `Document failed validation: ${violations.map((v) => v.message).join("; ")}`);
    }
  }
}
