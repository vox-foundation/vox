/**
 * FableForge — Cascade Delete, Auth Guard, and Siloing Spec
 *
 * Drop into: convex/games/mutations.ts (replace the existing deleteGame mutation)
 *            convex/lib/auth/requireGameOwner.ts (new file)
 *            tests/integration/auth/game-siloing.test.ts (new file)
 *
 * Implements: T-191 (cascade delete), T-192 (soft-delete), T-193 (auth audit),
 *             T-195 (userGameLibrary cascade), T-205 (siloing tests)
 *
 * IMPORTANT before merging:
 *   1. Verify all table names match your convex/schema.ts (e.g. "saves" vs "gameSaves").
 *   2. The R2 blob cleanup is async — schedule it via a Convex scheduled function,
 *      not inline in the mutation, to avoid timeout on large games.
 *   3. Confirm the `games` table has `deletedAt` before deploying T-192.
 */

// ─── convex/lib/auth/requireGameOwner.ts ──────────────────────────────────────
//
// Pattern: every mutation that touches game data calls this first.
// Throws ConvexError if the caller is not the owner (or an admin).
//
// Add an ESLint rule (T-193) that flags any mutation touching `gameId` that
// does NOT call requireGameOwnerOrAdmin — this becomes a CI lint gate.

import { ConvexError } from "convex/values";
import type { MutationCtx, QueryCtx } from "../_generated/server";

export async function requireGameOwnerOrAdmin(
  ctx: MutationCtx | QueryCtx,
  gameId: string,
): Promise<void> {
  const identity = await ctx.auth.getUserIdentity();
  if (!identity) {
    throw new ConvexError({ code: "UNAUTHENTICATED", message: "Must be logged in." });
  }

  const game = await ctx.db.get(gameId as unknown as import("convex/values").Id<"games">);
  if (!game) {
    throw new ConvexError({ code: "NOT_FOUND", message: "Game not found." });
  }

  const user = await ctx.db
    .query("users")
    .withIndex("by_clerk_id", (q) => q.eq("clerkId", identity.subject))
    .unique();

  if (!user) {
    throw new ConvexError({ code: "UNAUTHENTICATED", message: "User record not found." });
  }

  const isOwner = game.userId === user._id;
  const isAdmin = user.role === "admin";

  if (!isOwner && !isAdmin) {
    throw new ConvexError({
      code: "FORBIDDEN",
      message: "You do not have permission to modify this game.",
      gameId,
    });
  }
}

// ─── convex/games/mutations.ts — deleteGame (replacement) ────────────────────
//
// Full cascade: deletes every child row and schedules R2 blob cleanup.
// Wraps the hard delete in a soft-delete if T-192 is enabled.

import { mutation, internalMutation } from "../_generated/server";
import { v } from "convex/values";
import { internal } from "../_generated/api";

/**
 * Soft-delete a game (T-192).
 * Sets deletedAt; a scheduled job hard-deletes after 30 days.
 * Shows a "Recover" option in the dashboard for 30 days.
 */
export const softDeleteGame = mutation({
  args: { gameId: v.id("games") },
  handler: async (ctx, { gameId }) => {
    await requireGameOwnerOrAdmin(ctx, gameId);
    await ctx.db.patch(gameId, { deletedAt: Date.now() });
  },
});

/**
 * Hard-delete a game and ALL child data (T-191).
 * Called directly in tests; called by the 30-day scheduled job in production.
 *
 * Tables purged (verify all exist in schema.ts):
 *   saves, characterLibrary, backgroundLibrary, characterAnchors,
 *   locationAnchors, userGameLibrary, gameDrafts, generationAttempts,
 *   audioForgeJobs, builds, generatedAssets, panels (denormalized cache)
 *
 * R2 blobs: scheduled async via internal.storage.deleteOrphanedBlobs
 */
export const hardDeleteGame = internalMutation({
  args: { gameId: v.id("games") },
  handler: async (ctx, { gameId }) => {
    // Collect R2 asset keys BEFORE deleting rows (needed for blob cleanup).
    // VERIFIED 2026-04-23: generatedAssets uses `assetKey` (string) + `r2Url`.
    // There is no `storageKey` field on this table.
    const assetKeys = await ctx.db
      .query("generatedAssets")
      .withIndex("by_game", (q) => q.eq("gameId", gameId))
      .collect()
      .then((rows) => rows.map((r) => r.assetKey).filter(Boolean));

    // ── Child table cascade ────────────────────────────────────────────────

    // VERIFIED 2026-04-23 against real schema — index names differ per table.
    // Format: { table, index } where index is the real index name in convex/schema.ts
    //
    // Tables that use "by_game"      : characterLibrary, backgroundLibrary, userGameLibrary,
    //                                  gameDrafts, generationAttempts, audioForgeJobs,
    //                                  generatedAssets, scenes
    // Tables that use "by_game_id"   : characters, builds
    // Tables with composite first-field: characterAnchors ("by_character" → [gameId, characterId])
    //                                    locationAnchors  ("by_location"  → [gameId, locationId])
    // Existing saves table            : "by_user_and_game" needs filter (see note below)
    // panels (T-027 cache)            : not yet in schema; add when that table is created

    const CASCADE_TABLES: Array<{ table: string; index: string }> = [
      { table: "characterLibrary",   index: "by_game" },
      { table: "backgroundLibrary",  index: "by_game" },
      { table: "userGameLibrary",    index: "by_game" },
      { table: "gameDrafts",         index: "by_game" },
      { table: "generationAttempts", index: "by_game" },
      { table: "audioForgeJobs",     index: "by_game" },
      { table: "generatedAssets",    index: "by_game" },
      { table: "characters",         index: "by_game_id" },   // NOT by_game — verified
      { table: "builds",             index: "by_game_id" },   // NOT by_game — verified
      { table: "characterAnchors",   index: "by_character" }, // prefix scan on gameId
      { table: "locationAnchors",    index: "by_location" },  // prefix scan on gameId
    ];

    for (const { table, index } of CASCADE_TABLES) {
      const rows = await (ctx.db as unknown as Record<
        string,
        { query: () => { withIndex: (name: string, fn: (q: unknown) => unknown) => { collect: () => Promise<Array<{ _id: unknown }>> } } }
      >)[table]
        .query()
        .withIndex(index, (q: unknown) => (q as { eq: (field: string, value: unknown) => unknown }).eq("gameId", gameId))
        .collect();

      for (const row of rows) {
        await ctx.db.delete(row._id as import("convex/values").Id<typeof table>);
      }
    }

    // saves (existing table) uses "by_user_and_game" with userId as the first field,
    // so we cannot use it as a gameId-only prefix. Use filter instead.
    // After PR 6 adds the new saves table with .index("by_game", ["gameId"]) this
    // can be moved into CASCADE_TABLES above.
    const existingSaves = await (ctx.db as any)
      .query("saves")
      .filter((q: any) => q.eq(q.field("gameId"), gameId))
      .collect();
    for (const row of existingSaves) {
      await ctx.db.delete((row as any)._id);
    }

    // ── Delete the game itself ─────────────────────────────────────────────
    await ctx.db.delete(gameId);

    // ── Schedule R2 blob cleanup (async, won't cause timeout) ─────────────
    if (assetKeys.length > 0) {
      await ctx.scheduler.runAfter(0, internal.storage.deleteOrphanedBlobs, {
        keys: assetKeys,
      });
    }
  },
});

/**
 * Public-facing deleteGame: soft-deletes for regular users (T-192).
 * Admins bypass soft-delete by calling hardDeleteGame directly.
 */
export const deleteGame = mutation({
  args: { gameId: v.id("games") },
  handler: async (ctx, { gameId }) => {
    await requireGameOwnerOrAdmin(ctx, gameId);
    await ctx.db.patch(gameId, { deletedAt: Date.now() });
    // The hard delete runs 30 days later via the scheduled cleanup job.
  },
});

// ─── Scheduled cleanup job (30-day retention) ─────────────────────────────────

export const purgeExpiredGames = internalMutation({
  args: {},
  handler: async (ctx) => {
    const thirtyDaysAgo = Date.now() - 30 * 24 * 60 * 60 * 1000;
    const expired = await ctx.db
      .query("games")
      .withIndex("by_deleted_at", (q) =>
        q.gt("deletedAt", 0).lt("deletedAt", thirtyDaysAgo),
      )
      .collect();

    for (const game of expired) {
      await ctx.runMutation(internal.games.mutations.hardDeleteGame, {
        gameId: game._id,
      });
    }
  },
});

// ─── ESLint rule stub (T-193) ─────────────────────────────────────────────────
//
// Add this rule to eslint-plugin-fableforge (create if doesn't exist):
//
//   rule: "require-game-owner-check"
//   description: "Every mutation that accepts a gameId argument must call
//                 requireGameOwnerOrAdmin before any db read or write."
//
// Implementation sketch:
//   - Selector: FunctionDeclaration, ArrowFunctionExpression inside mutation({...})
//   - Check: if params include `gameId`, ensure requireGameOwnerOrAdmin appears
//     in the function body BEFORE any ctx.db call.
//   - Report: "mutation touches gameId without auth guard."
//
// This rule runs in CI (pnpm eslint --rule fableforge/require-game-owner-check)
// and blocks merge on violation (T-193 acceptance criterion).

// ─── tests/integration/auth/game-siloing.test.ts ─────────────────────────────
//
// Test matrix: two users (A, B); game owned by A.
// Every mutation that accepts gameId is called by B; all must throw FORBIDDEN.
//
// Mutations to cover (update this list as new mutations are added):
//   deleteGame, softDeleteGame, updateGame, publishDraft, updateDraft,
//   createPanel, updatePanel, deletePanel, movePanel,
//   regenerateBeat, batchGenerate, createCharacter, deleteCharacter,
//   addSave, loadSave, deleteSave
//
// Test structure (Vitest + convex-test or a local Convex instance):

/*
import { describe, test, expect, beforeEach } from "vitest";
import { createTestConvex } from "../../helpers/convex-test";
import { api } from "../../../convex/_generated/api";

describe("Game siloing", () => {
  let convex: ReturnType<typeof createTestConvex>;
  let userA: { token: string; userId: string };
  let userB: { token: string; userId: string };
  let gameId: string;

  beforeEach(async () => {
    convex = createTestConvex();
    userA = await convex.createUser("user-a@test.com");
    userB = await convex.createUser("user-b@test.com");
    gameId = await convex.mutation(api.games.mutations.createGame, {
      title: "User A's Game",
    }, { token: userA.token });
  });

  const MUTATIONS_UNDER_TEST = [
    ["deleteGame",   { gameId }],
    ["updateGame",   { gameId, title: "Hacked" }],
    ["publishDraft", { gameId }],
    // ... add all mutations that accept gameId
  ] as const;

  for (const [mutName, args] of MUTATIONS_UNDER_TEST) {
    test(`User B cannot call ${mutName} on User A's game`, async () => {
      await expect(
        convex.mutation(api.games.mutations[mutName], args, { token: userB.token }),
      ).rejects.toMatchObject({ code: "FORBIDDEN" });
    });
  }

  test("User B cannot read User A's private game", async () => {
    const result = await convex.query(api.games.queries.getGame, { gameId }, {
      token: userB.token,
    });
    expect(result).toBeNull();
  });

  test("Admin can call deleteGame on any game", async () => {
    const admin = await convex.createUser("admin@test.com", { role: "admin" });
    await expect(
      convex.mutation(api.games.mutations.deleteGame, { gameId }, { token: admin.token }),
    ).resolves.not.toThrow();
  });
});
*/

// ─── schema.ts additions required ────────────────────────────────────────────
//
// To support the soft-delete and the scheduled cleanup query:
//
//   games: defineTable({
//     ...existing fields...,
//     deletedAt: v.optional(v.number()),   // T-192: null = not deleted
//   })
//   .index("by_deleted_at", ["deletedAt"]) // required by purgeExpiredGames
//
// Ensure all child tables have:
//   .index("by_game", ["gameId"])          // required by hardDeleteGame cascade
