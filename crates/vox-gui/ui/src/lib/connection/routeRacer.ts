/**
 * 6-tier staggered route racing (`150ms` step delay) + `300s` failure cooldown (`T3-WEBHOOK-IMPORT-04`).
 */

export const ROUTE_RACE_STEP_DELAY_MS = 150;
export const ROUTE_FAILURE_COOLDOWN_MS = 300_000;

export type ConnectionRouteTier =
  | 'last_working'
  | 'tailscale_MagicDNS'
  | 'lan_direct'
  | 'localhost_loopback'
  | 'public_relay'
  | 'manual_fallback';

export interface RouteCandidate {
  readonly tier: ConnectionRouteTier;
  readonly url: string;
}

const TIER_PRIORITY: Readonly<Record<ConnectionRouteTier, number>> = {
  last_working: 0,
  tailscale_MagicDNS: 1,
  lan_direct: 2,
  localhost_loopback: 3,
  public_relay: 4,
  manual_fallback: 5,
};

export interface StaggeredRaceScheduleEntry {
  readonly candidate: RouteCandidate;
  readonly startDelayMs: number;
}

export class ConnectionRouteRacer {
  private readonly failedUntilMs = new Map<string, number>();

  public recordRouteFailure(url: string, nowMs: number): void {
    this.failedUntilMs.set(url, nowMs + ROUTE_FAILURE_COOLDOWN_MS);
  }

  public isRouteCoolingDown(url: string, nowMs: number): boolean {
    const exp = this.failedUntilMs.get(url);
    if (exp === undefined) {
      return false;
    }
    if (nowMs >= exp) {
      this.failedUntilMs.delete(url);
      return false;
    }
    return true;
  }

  public planStaggeredRace(
    candidates: readonly RouteCandidate[],
    nowMs: number,
  ): readonly StaggeredRaceScheduleEntry[] {
    const eligible = candidates.filter((c) => !this.isRouteCoolingDown(c.url, nowMs));
    const sorted = [...eligible].sort(
      (a, b) => TIER_PRIORITY[a.tier] - TIER_PRIORITY[b.tier],
    );
    return sorted.map((candidate, idx) => ({
      candidate,
      startDelayMs: idx * ROUTE_RACE_STEP_DELAY_MS,
    }));
  }
}
