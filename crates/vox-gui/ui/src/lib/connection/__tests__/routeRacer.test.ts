import { describe, expect, it } from 'vitest';
import {
  ConnectionRouteRacer,
  ROUTE_FAILURE_COOLDOWN_MS,
  ROUTE_RACE_STEP_DELAY_MS,
} from '../routeRacer';

describe('ConnectionRouteRacer (T3-WEBHOOK-IMPORT-04)', () => {
  it('orders 6 tiers with 150ms staggered start delays and enforces 300s failure cooldown', () => {
    const racer = new ConnectionRouteRacer();
    const candidates = [
      { tier: 'localhost_loopback' as const, url: 'ws://127.0.0.1:4100' },
      { tier: 'last_working' as const, url: 'ws://100.64.0.2:4100' },
      { tier: 'lan_direct' as const, url: 'ws://192.168.1.10:4100' },
    ];
    const schedule = racer.planStaggeredRace(candidates, 1_000);
    expect(schedule.map((s) => s.candidate.tier)).toEqual([
      'last_working',
      'lan_direct',
      'localhost_loopback',
    ]);
    expect(schedule.map((s) => s.startDelayMs)).toEqual([
      0,
      ROUTE_RACE_STEP_DELAY_MS,
      2 * ROUTE_RACE_STEP_DELAY_MS,
    ]);

    racer.recordRouteFailure('ws://100.64.0.2:4100', 1_000);
    const duringCooldown = racer.planStaggeredRace(candidates, 2_000);
    expect(duringCooldown.map((s) => s.candidate.tier)).toEqual([
      'lan_direct',
      'localhost_loopback',
    ]);

    const afterCooldown = racer.planStaggeredRace(
      candidates,
      1_000 + ROUTE_FAILURE_COOLDOWN_MS + 1,
    );
    expect(afterCooldown).toHaveLength(3);
  });
});
