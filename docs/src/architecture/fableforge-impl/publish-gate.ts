/**
 * Publish Gate + Cloud Saves
 *
 * Drop into:
 *   convex/games/studio/actions.ts     (publishDraft replacement — T-115, T-116, T-117)
 *   convex/runtime/saves.ts            (cloud save mutations — T-206, T-207, T-209)
 *
 * Implements: T-115 (playtest guard), T-116 (publish checklist), T-117 (unreachable branch),
 *             T-206 (cloud save mutations), T-207 (runtime writes to Convex),
 *             T-209 (autosave every N panels)
 *
 * IMPORTANT before merging:
 *   1. Wire the linter import to your real package path.
 *   2. Confirm "saves" table exists in schema.ts (see schema additions at bottom).
 *   3. The playtestToken mechanism requires the wizard to generate a token when
 *      the user completes playtest; verify the UI flow wires this correctly.
 *
 * SCHEMA ADDITIONS REQUIRED ON gameDrafts (verified 2026-04-23 against real schema):
 *   These fields are referenced below but do NOT exist on gameDrafts yet.
 *   Add them as part of PR 2 or PR 5:
 *
 *   gameDrafts: defineTable({
 *     ...existing fields...,
 *     coverImageKey:  v.optional(v.string()),  // R2 key for the cover thumbnail
 *     contentRating:  v.optional(v.union(
 *       v.literal("sfw"), v.literal("pg13"), v.literal("r18")
 *     )),
 *     // NOTE: games.contentRating uses "general" | "mature"; translate at publish time:
 *     //   sfw/pg13 → "general", r18 → "mature"
 *   })
 */

import { action, mutation, query } from "../_generated/server";
import { ConvexError } from "convex/values";
import { v } from "convex/values";
import { LintEngine } from "@fableforge/ffscript/linter";       // adjust path
import { FFScriptDoc } from "@fableforge/ffscript/mutations";   // adjust path
import { internal } from "../_generated/api";

// ─── Publish checklist result (T-116) ────────────────────────────────────────

interface ChecklistItem {
  id:     string;
  label:  string;
  passed: boolean;
  detail?: string;
}

interface PublishChecklist {
  items:     ChecklistItem[];
  allPassed: boolean;
}

function buildChecklist(doc: ReturnType<FFScriptDoc["state"]>, opts: {
  hasCoverImage:   boolean;
  hasPlaytested:   boolean;
  panelCount:      number;
}): PublishChecklist {
  // Run linter for errors
  const engine = new LintEngine({ errorsOnly: true });
  const report = engine.lint(doc as Parameters<typeof engine.lint>[0]);

  // Reachability: every panel should be reachable (linter rule already checks this)
  const unreachableFindings = report.findings.filter(
    (f) => f.ruleId === "ff/flow/unreachable-node",
  );

  const panelsWithBg    = (doc.panels as Array<{ background: unknown }>).filter((p) => p.background != null).length;
  const panelsWithSpeaker = (doc.panels as Array<{ bubbles: Array<{ style: string }> }>)
    .flatMap((p) => p.bubbles)
    .filter((b) => b.style !== "caption").length;

  const items: ChecklistItem[] = [
    {
      id:     "has-panels",
      label:  "Game has at least one panel",
      passed: opts.panelCount > 0,
    },
    {
      id:     "all-backgrounds",
      label:  "All panels have a background image",
      passed: panelsWithBg === opts.panelCount,
      detail: panelsWithBg < opts.panelCount
        ? `${opts.panelCount - panelsWithBg} panel(s) missing background` : undefined,
    },
    {
      id:     "no-lint-errors",
      label:  "No linter errors",
      passed: !report.hasErrors,
      detail: report.hasErrors
        ? `${report.errorCount} error(s): ${report.findings.slice(0, 3).map((f) => f.message).join("; ")}` : undefined,
    },
    {
      id:     "no-unreachable-branches",
      label:  "All branches are reachable",
      passed: unreachableFindings.length === 0,
      detail: unreachableFindings.length > 0
        ? `${unreachableFindings.length} unreachable node(s)` : undefined,
    },
    {
      id:     "has-cover-image",
      label:  "Cover image is set",
      passed: opts.hasCoverImage,
    },
    {
      id:     "has-playtested",
      label:  "Game has been playtested",
      passed: opts.hasPlaytested,
      detail: opts.hasPlaytested ? undefined : "Click 'Playtest' before publishing",
    },
  ];

  return { items, allPassed: items.every((i) => i.passed) };
}

// ─── publishDraft (T-115 / T-116 / T-117) ────────────────────────────────────

export const publishDraft = action({
  args: {
    draftId:       v.id("gameDrafts"),
    playtestToken: v.optional(v.string()), // issued by the playtest session
  },
  handler: async (ctx, { draftId, playtestToken }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new ConvexError({ code: "UNAUTHENTICATED" });

    // Load draft
    const draft = await ctx.runQuery(internal.games.queries.getDraft, { draftId });
    if (!draft) throw new ConvexError({ code: "NOT_FOUND", message: "Draft not found" });

    if (draft.userId !== identity.subject) {
      throw new ConvexError({ code: "FORBIDDEN" });
    }

    // Load and open the FFScript document
    const rawDoc = await ctx.runAction(internal.storage.loadFFScript, {
      key: draft.ffscriptKey,
    });
    const doc = FFScriptDoc.open(rawDoc);

    // Check playtest token (T-115)
    const hasPlaytested = Boolean(
      playtestToken && await ctx.runQuery(internal.playtest.validateToken, {
        draftId, token: playtestToken,
      }),
    );

    // Run checklist (T-116)
    const checklist = buildChecklist(doc.state, {
      hasCoverImage: Boolean(draft.coverImageKey),
      hasPlaytested,
      panelCount:    doc.state.panels.length,
    });

    if (!checklist.allPassed) {
      throw new ConvexError({
        code:      "PUBLISH_CHECKLIST_FAILED",
        message:   "Game is not ready to publish.",
        checklist: checklist.items,
      });
    }

    // Generate a globally unique slug (T-194)
    const baseSlug     = slugify(draft.title);
    const publishedSlug = await ctx.runMutation(internal.games.mutations.claimSlug, { baseSlug });

    // Create or update the games row
    const gameId = await ctx.runMutation(internal.games.mutations.upsertPublishedGame, {
      draftId,
      publishedSlug,
      ffscriptKey:   draft.ffscriptKey,
      coverImageKey: draft.coverImageKey,
      title:         draft.title,
      contentRating: draft.contentRating,
      publishedAt:   Date.now(),
    });

    // Mark draft as published
    await ctx.runMutation(internal.games.mutations.markDraftPublished, { draftId, gameId });

    return { gameId, publishedSlug, checklist };
  },
});

// ─── Slug generation (T-194) ─────────────────────────────────────────────────

function slugify(title: string): string {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9]+/g, "-")
    .replace(/^-+|-+$/g, "")
    .slice(0, 60);
}

// ─── Cloud save mutations (T-206 / T-207 / T-209) ────────────────────────────

const MAX_MANUAL_SLOTS  = 20;
const MAX_AUTO_SLOTS    = 5;
const AUTOSAVE_INTERVAL = 5; // panels visited before autosave

// Save state shape (mirrors timeline-save.ts — consolidates localStorage → Convex)
const SaveStateSchema = v.object({
  currentPanelId: v.string(),
  variables:      v.record(v.string(), v.union(v.string(), v.number(), v.boolean())),
  /** Ordered list of panel IDs the player has visited. */
  history:        v.array(v.string()),
  seed:           v.optional(v.number()),
  playedPercent:  v.optional(v.number()),
});

/** Write a manual save to a named slot (0–19). Overwrites if slot exists. */
export const saveSlot = mutation({
  args: {
    gameId:       v.id("games"),
    slot:         v.number(),
    state:        SaveStateSchema,
    screenshotKey: v.optional(v.string()), // R2 key for the screenshot thumbnail (T-211)
  },
  handler: async (ctx, { gameId, slot, state, screenshotKey }) => {
    if (slot < 0 || slot >= MAX_MANUAL_SLOTS) {
      throw new ConvexError({ code: "INVALID_SLOT", message: `Slot must be 0–${MAX_MANUAL_SLOTS - 1}` });
    }

    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new ConvexError({ code: "UNAUTHENTICATED" });

    const user = await ctx.db
      .query("users")
      .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
      .unique();
    if (!user) throw new ConvexError({ code: "NOT_FOUND" });

    const existing = await ctx.db
      .query("saves")
      .withIndex("by_user_game_slot", (q) =>
        q.eq("userId", user._id).eq("gameId", gameId).eq("slot", slot),
      )
      .unique();

    const saveData = {
      userId:        user._id,
      gameId,
      slot,
      isAutosave:    false,
      state,
      screenshotKey: screenshotKey ?? null,
      savedAt:       Date.now(),
    };

    if (existing) {
      await ctx.db.patch(existing._id, saveData);
    } else {
      await ctx.db.insert("saves", saveData);
    }
  },
});

/** Write an autosave. Rotates through MAX_AUTO_SLOTS slots (slot 100–104 internally). */
export const autosave = mutation({
  args: {
    gameId: v.id("games"),
    state:  SaveStateSchema,
    panelsVisited: v.number(), // trigger threshold check on client
  },
  handler: async (ctx, { gameId, state }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new ConvexError({ code: "UNAUTHENTICATED" });

    const user = await ctx.db
      .query("users")
      .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
      .unique();
    if (!user) throw new ConvexError({ code: "NOT_FOUND" });

    // Find the oldest autosave slot to overwrite
    const autoSlots = await ctx.db
      .query("saves")
      .withIndex("by_user_game_slot", (q) =>
        q.eq("userId", user._id).eq("gameId", gameId),
      )
      .filter((q) => q.eq(q.field("isAutosave"), true))
      .collect();

    const nextSlot = autoSlots.length < MAX_AUTO_SLOTS
      ? 100 + autoSlots.length
      : autoSlots.sort((a, b) => a.savedAt - b.savedAt)[0]!.slot; // LRU eviction

    const existing = autoSlots.find((s) => s.slot === nextSlot);
    const saveData = {
      userId:        user._id,
      gameId,
      slot:          nextSlot,
      isAutosave:    true,
      state,
      screenshotKey: null,
      savedAt:       Date.now(),
    };

    if (existing) {
      await ctx.db.patch(existing._id, saveData);
    } else {
      await ctx.db.insert("saves", saveData);
    }
  },
});

/** Load a specific save slot. */
export const loadSlot = query({
  args: { gameId: v.id("games"), slot: v.number() },
  handler: async (ctx, { gameId, slot }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) return null;

    const user = await ctx.db
      .query("users")
      .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
      .unique();
    if (!user) return null;

    return ctx.db
      .query("saves")
      .withIndex("by_user_game_slot", (q) =>
        q.eq("userId", user._id).eq("gameId", gameId).eq("slot", slot),
      )
      .unique();
  },
});

/** List all saves for a game (manual + auto), sorted newest-first. */
export const listSlots = query({
  args: { gameId: v.id("games") },
  handler: async (ctx, { gameId }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) return [];

    const user = await ctx.db
      .query("users")
      .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
      .unique();
    if (!user) return [];

    const saves = await ctx.db
      .query("saves")
      .withIndex("by_user_game_slot", (q) =>
        q.eq("userId", user._id).eq("gameId", gameId),
      )
      .collect();

    return saves.sort((a, b) => b.savedAt - a.savedAt);
  },
});

/** Delete a save slot. */
export const deleteSlot = mutation({
  args: { gameId: v.id("games"), slot: v.number() },
  handler: async (ctx, { gameId, slot }) => {
    const identity = await ctx.auth.getUserIdentity();
    if (!identity) throw new ConvexError({ code: "UNAUTHENTICATED" });

    const user = await ctx.db
      .query("users")
      .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
      .unique();
    if (!user) throw new ConvexError({ code: "NOT_FOUND" });

    const save = await ctx.db
      .query("saves")
      .withIndex("by_user_game_slot", (q) =>
        q.eq("userId", user._id).eq("gameId", gameId).eq("slot", slot),
      )
      .unique();

    if (save) await ctx.db.delete(save._id);
  },
});

// ─── Runtime autosave hook (T-207 / T-209) ────────────────────────────────────
//
// In src/lib/runtime/FFScriptPlayer.ts, add:
//
//   private _panelsSinceLastSave = 0;
//
//   onPanelEnter(panelId: string): void {
//     this._panelsSinceLastSave++;
//     if (this._panelsSinceLastSave >= AUTOSAVE_INTERVAL) {
//       this._panelsSinceLastSave = 0;
//       void convex.mutation(api.runtime.saves.autosave, {
//         gameId: this._gameId,
//         state:  this.serialize(),
//         panelsVisited: this._totalPanelsVisited,
//       });
//     }
//   }
//
// The AUTOSAVE_INTERVAL constant (5 panels) is defined above and can be made
// user-configurable via a game settings field.

// ─── schema.ts additions required ────────────────────────────────────────────
//
//   saves: defineTable({
//     userId:        v.id("users"),
//     gameId:        v.id("games"),
//     slot:          v.number(),         // 0–19 manual, 100–104 autosave
//     isAutosave:    v.boolean(),
//     state: v.object({
//       currentPanelId: v.string(),
//       variables:      v.record(v.string(), v.union(v.string(), v.number(), v.boolean())),
//       history:        v.array(v.string()),
//       seed:           v.optional(v.number()),
//       playedPercent:  v.optional(v.number()),
//     }),
//     screenshotKey: v.union(v.string(), v.null()),
//     savedAt:       v.number(),
//   })
//   .index("by_game",           ["gameId"])
//   .index("by_user_game_slot", ["userId", "gameId", "slot"]),
