/**
 * FableForge — Generation Orchestrator (ImageOrchestrator)
 *
 * Drop into: convex/lib/generation/orchestrator.ts
 *
 * Implements: T-004 (wire selectedProvider), T-129 (cross-provider fallback),
 *             T-130 (circuit breaker), T-007 (provider/tier stamps on assets)
 *
 * Architecture inspired by Vox's ModelScorer + ScoringWeights pattern:
 *   - Providers are scored per-request, not hardcoded.
 *   - ScoringWeights are tunable without code changes.
 *   - Circuit breaker state lives in Convex (providerHealth table).
 *
 * IMPORTANT before merging:
 *   1. Verify provider client imports match your actual provider files.
 *   2. Ensure `providerHealth` table exists in convex/schema.ts (see bottom).
 *   3. Wire selectedProvider from gameDrafts.selectedProvider (T-004 fix).
 */

// ─── Provider identifiers ─────────────────────────────────────────────────────

export type ProviderId = "fal" | "comfyui" | "replicate";

export const PROVIDER_PRIORITY: ProviderId[] = ["comfyui", "fal", "replicate"];

// ─── Generation request ───────────────────────────────────────────────────────

export interface GenerationRequest {
  /** Desired provider (from gameDrafts.selectedProvider). null = auto-select. */
  preferredProvider:  ProviderId | null;
  /** The image-model task type (drives capability matching). */
  task:               "text2image" | "inpaint" | "img2img" | "upscale" | "controlnet";
  /** Width × height of the output. Derived from panel.aspectRatio (T-031). */
  dims:               { w: number; h: number };
  /** Full generation prompt. */
  prompt:             string;
  negativePrompt?:    string;
  /** Image input (required for inpaint / img2img). */
  initImage?:         string; // R2 key or data URL
  mask?:              string; // R2 key for inpaint mask (T-125)
  /** LoRA weights (T-131). */
  loras?:             Array<{ name: string; weight: number }>;
  /** Pose reference image R2 key (T-135). */
  poseReferenceImage?: string;
  /** Seed. null = random. */
  seed?:              number | null;
  /** Content tier — routes to appropriate model variant (T-149). */
  contentRating:      "sfw" | "pg13" | "r18";
  /** Correlation ID for tracing. */
  correlationId:      string;
  /** Game ID — stamped on the resulting asset row (T-007). */
  gameId:             string;
  panelId?:           string;
}

// ─── Generation result ────────────────────────────────────────────────────────

export interface GenerationResult {
  /** R2 key of the stored output image. */
  assetKey:     string;
  providerId:   ProviderId;
  modelId:      string;
  workflowHash: string;
  seed:         number;
  /** Cost in USD. Stored on generatedAssets for billing (T-259). */
  costUsd:      number;
  /** Attempt number (1 = first try, 2 = after first failure, etc.). */
  attempt:      number;
  durationMs:   number;
}

// ─── Scoring weights (Vox ModelScorer pattern) ───────────────────────────────
//
// Tune these to change routing behavior without code changes.
// Override per-request by passing ScoringWeights to ImageOrchestrator.

export interface ScoringWeights {
  /** Bonus for the user's preferred provider. */
  preferredProviderBonus: number;
  /** Penalty for each percentage point of error rate in the circuit-breaker window. */
  errorRatePenaltyPerPct: number;
  /** Bonus for providers with the capability needed by the task. */
  capabilityMatchBonus:   number;
  /** Bonus for the cheapest provider (per image). */
  costEfficiencyBonus:    number;
  /** Penalty for providers whose P95 latency exceeds 20 s. */
  highLatencyPenalty:     number;
}

export const DEFAULT_SCORING_WEIGHTS: ScoringWeights = {
  preferredProviderBonus: 100,
  errorRatePenaltyPerPct: 2,
  capabilityMatchBonus:   50,
  costEfficiencyBonus:    20,
  highLatencyPenalty:     30,
};

// ─── Provider capabilities registry ──────────────────────────────────────────

type ProviderCapabilities = {
  tasks:        GenerationRequest["task"][];
  supportsLora: boolean;
  supportsR18:  boolean;
  avgCostUsd:   number; // per 1024×1024 image, approx
  avgLatencyMs: number; // P50
};

const PROVIDER_CAPS: Record<ProviderId, ProviderCapabilities> = {
  comfyui: {
    tasks:        ["text2image", "inpaint", "img2img", "upscale", "controlnet"],
    supportsLora: true,
    supportsR18:  true,
    avgCostUsd:   0.003,
    avgLatencyMs: 8_000,
  },
  fal: {
    tasks:        ["text2image", "inpaint", "img2img"],
    supportsLora: false,
    supportsR18:  false,
    avgCostUsd:   0.008,
    avgLatencyMs: 5_000,
  },
  replicate: {
    tasks:        ["text2image", "img2img", "upscale"],
    supportsLora: true,
    supportsR18:  false,
    avgCostUsd:   0.012,
    avgLatencyMs: 12_000,
  },
};

// ─── Circuit breaker state (Convex providerHealth table) ─────────────────────

export interface ProviderHealthSnapshot {
  providerId:       ProviderId;
  errorRatePct:     number; // rolling 5-minute window
  openUntil:        number | null; // epoch ms; null = closed (healthy)
  lastChecked:      number; // epoch ms
}

const CIRCUIT_OPEN_THRESHOLD_PCT = 40;    // open if error rate > 40%
const CIRCUIT_OPEN_DURATION_MS   = 10 * 60 * 1000; // 10 minutes

export function isCircuitOpen(health: ProviderHealthSnapshot): boolean {
  if (health.openUntil === null) return false;
  return Date.now() < health.openUntil;
}

export function shouldOpenCircuit(health: ProviderHealthSnapshot): boolean {
  return health.errorRatePct > CIRCUIT_OPEN_THRESHOLD_PCT;
}

export function openCircuit(health: ProviderHealthSnapshot): ProviderHealthSnapshot {
  return {
    ...health,
    openUntil: Date.now() + CIRCUIT_OPEN_DURATION_MS,
  };
}

// ─── Scoring function ─────────────────────────────────────────────────────────

export function scoreProvider(
  provider: ProviderId,
  request:  GenerationRequest,
  health:   ProviderHealthSnapshot,
  weights:  ScoringWeights = DEFAULT_SCORING_WEIGHTS,
): number {
  const caps = PROVIDER_CAPS[provider];
  let score  = 0;

  // Hard disqualifications — return -Infinity immediately
  if (isCircuitOpen(health)) return -Infinity;
  if (!caps.tasks.includes(request.task)) return -Infinity;
  if (request.contentRating === "r18" && !caps.supportsR18) return -Infinity;
  if (request.loras && request.loras.length > 0 && !caps.supportsLora) return -Infinity;

  // Preferred provider bonus
  if (request.preferredProvider === provider) {
    score += weights.preferredProviderBonus;
  }

  // Capability match
  score += weights.capabilityMatchBonus;

  // Cost efficiency (invert: lower cost = higher score)
  const cheapest = Math.min(...Object.values(PROVIDER_CAPS).map((c) => c.avgCostUsd));
  if (caps.avgCostUsd === cheapest) score += weights.costEfficiencyBonus;

  // Error rate penalty
  score -= health.errorRatePct * weights.errorRatePenaltyPerPct;

  // Latency penalty
  if (caps.avgLatencyMs > 20_000) score -= weights.highLatencyPenalty;

  return score;
}

// ─── ImageOrchestrator ────────────────────────────────────────────────────────

export interface OrchestratorDeps {
  /**
   * Fetch health snapshots for all providers from Convex.
   * In a mutation context: ctx.db.query("providerHealth").collect()
   */
  getProviderHealth: () => Promise<ProviderHealthSnapshot[]>;

  /**
   * Persist a health update after an attempt.
   */
  updateProviderHealth: (snapshot: ProviderHealthSnapshot) => Promise<void>;

  /**
   * Record a generation attempt for analytics (T-007, T-259).
   */
  recordAttempt: (attempt: {
    correlationId: string;
    providerId:    ProviderId;
    modelId:       string;
    success:       boolean;
    durationMs:    number;
    costUsd:       number;
    errorCode?:    string;
  }) => Promise<void>;

  /**
   * The actual generation call for a given provider.
   * Throws on failure (provider error, timeout, rate limit).
   */
  generate: (provider: ProviderId, request: GenerationRequest) => Promise<GenerationResult>;
}

export class ImageOrchestrator {
  constructor(
    private readonly deps: OrchestratorDeps,
    private readonly weights: ScoringWeights = DEFAULT_SCORING_WEIGHTS,
  ) {}

  async generate(request: GenerationRequest): Promise<GenerationResult> {
    const healthMap = await this._loadHealthMap();
    const ranked    = this._rankProviders(request, healthMap);

    if (ranked.length === 0) {
      throw new Error(
        `No eligible provider for task="${request.task}", rating="${request.contentRating}".` +
        " All providers are either circuit-open or lack the required capability.",
      );
    }

    let lastError: unknown;
    for (let attempt = 0; attempt < ranked.length; attempt++) {
      const provider = ranked[attempt]!;
      const t0       = Date.now();
      try {
        const result = await this.deps.generate(provider, request);
        const durationMs = Date.now() - t0;

        await this.deps.recordAttempt({
          correlationId: request.correlationId,
          providerId:    provider,
          modelId:       result.modelId,
          success:       true,
          durationMs,
          costUsd:       result.costUsd,
        });

        // Update health: success reduces error pressure
        await this._recordSuccess(provider, healthMap);

        return { ...result, attempt: attempt + 1 };

      } catch (err) {
        const durationMs = Date.now() - t0;
        await this.deps.recordAttempt({
          correlationId: request.correlationId,
          providerId:    provider,
          modelId:       "unknown",
          success:       false,
          durationMs,
          costUsd:       0,
          errorCode:     String(err),
        });

        await this._recordFailure(provider, healthMap);
        lastError = err;
        // continue to next provider
      }
    }

    throw new Error(
      `All ${ranked.length} provider(s) failed for task="${request.task}". Last error: ${lastError}`,
    );
  }

  private _rankProviders(
    request:   GenerationRequest,
    healthMap: Map<ProviderId, ProviderHealthSnapshot>,
  ): ProviderId[] {
    return PROVIDER_PRIORITY
      .map((p) => ({
        provider: p,
        score:    scoreProvider(p, request, healthMap.get(p) ?? this._defaultHealth(p), this.weights),
      }))
      .filter((x) => x.score > -Infinity)
      .sort((a, b) => b.score - a.score)
      .map((x) => x.provider);
  }

  private async _loadHealthMap(): Promise<Map<ProviderId, ProviderHealthSnapshot>> {
    const snapshots = await this.deps.getProviderHealth();
    const map = new Map<ProviderId, ProviderHealthSnapshot>();
    for (const s of snapshots) map.set(s.providerId, s);
    return map;
  }

  private _defaultHealth(provider: ProviderId): ProviderHealthSnapshot {
    return { providerId: provider, errorRatePct: 0, openUntil: null, lastChecked: 0 };
  }

  private async _recordSuccess(
    provider:  ProviderId,
    healthMap: Map<ProviderId, ProviderHealthSnapshot>,
  ): Promise<void> {
    const current = healthMap.get(provider) ?? this._defaultHealth(provider);
    // Decay error rate: success counts as 0% error
    const updated: ProviderHealthSnapshot = {
      ...current,
      errorRatePct: Math.max(0, current.errorRatePct - 5),
      openUntil:    current.openUntil,
      lastChecked:  Date.now(),
    };
    await this.deps.updateProviderHealth(updated);
  }

  private async _recordFailure(
    provider:  ProviderId,
    healthMap: Map<ProviderId, ProviderHealthSnapshot>,
  ): Promise<void> {
    const current = healthMap.get(provider) ?? this._defaultHealth(provider);
    const newRate  = Math.min(100, current.errorRatePct + 15);
    const snapshot: ProviderHealthSnapshot = {
      ...current,
      errorRatePct: newRate,
      lastChecked:  Date.now(),
      openUntil:    newRate > CIRCUIT_OPEN_THRESHOLD_PCT
        ? Date.now() + CIRCUIT_OPEN_DURATION_MS
        : current.openUntil,
    };
    await this.deps.updateProviderHealth(snapshot);
  }
}

// ─── Usage in convex/lib/generation/batchGeneration.ts ───────────────────────
//
// Replace the current hardcoded FAL call with:
//
//   const orchestrator = new ImageOrchestrator({
//     getProviderHealth: () =>
//       ctx.db.query("providerHealth").collect(),
//     updateProviderHealth: (snap) =>
//       ctx.db
//         .query("providerHealth")
//         .withIndex("by_provider", (q) => q.eq("providerId", snap.providerId))
//         .unique()
//         .then((row) =>
//           row
//             ? ctx.db.patch(row._id, snap)
//             : ctx.db.insert("providerHealth", snap),
//         ),
//     recordAttempt: (attempt) =>
//       ctx.db.insert("generationAttempts", { ...attempt, gameId, panelId }),
//     generate: (provider, req) => dispatchToProvider(provider, req, ctx),
//   });
//
//   const result = await orchestrator.generate({
//     preferredProvider: draft.selectedProvider ?? null, // ← T-004 fix
//     task:              "text2image",
//     dims:              aspectRatioDims(panel.aspectRatio),
//     prompt:            beat.visualPrompt,
//     seed:              panel.metadata.lockedSeed,
//     contentRating:     draft.contentRating,
//     correlationId:     `${gameId}-${panelId}-${Date.now()}`,
//     gameId,
//     panelId,
//   });

// ─── schema.ts additions required ────────────────────────────────────────────
//
//   providerHealth: defineTable({
//     providerId:   v.string(),           // "fal" | "comfyui" | "replicate"
//     errorRatePct: v.number(),
//     openUntil:    v.union(v.number(), v.null()),
//     lastChecked:  v.number(),
//   })
//   .index("by_provider", ["providerId"]),
//
//   generationAttempts: defineTable({
//     correlationId: v.string(),
//     providerId:    v.string(),
//     modelId:       v.string(),
//     success:       v.boolean(),
//     durationMs:    v.number(),
//     costUsd:       v.number(),
//     errorCode:     v.optional(v.string()),
//     gameId:        v.string(),
//     panelId:       v.optional(v.string()),
//   })
//   .index("by_game", ["gameId"])
//   .index("by_provider", ["providerId"]),
