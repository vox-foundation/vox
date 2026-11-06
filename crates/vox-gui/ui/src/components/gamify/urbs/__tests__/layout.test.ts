// crates/vox-gui/ui/src/components/gamify/urbs/__tests__/layout.test.ts
import { describe, it, expect } from 'vitest';
import { computeAttentionMultiplier, layoutTown } from '../layout';
import type { TownScan } from '../types';

describe('attention-weighted layout (Phase 261: URBS-ATTENTION-LAYOUT)', () => {
  const sampleScan: TownScan = {
    root: '/repo',
    scanned_at_ms: 0,
    truncated: false,
    crates: [
      {
        name: 'billing-api',
        root: 'services/billing-api',
        files: [
          {
            path: 'services/billing-api/src/invoice.ts',
            lines: 180,
            git_status: 'M',
            diff_added_lines: 42,
            diff_deleted_lines: 11,
          },
          {
            path: 'services/billing-api/src/stripe.ts',
            lines: 210,
            git_status: 'M',
            diff_added_lines: 18,
            diff_deleted_lines: 4,
          },
          ...Array.from({ length: 8 }, (_, i) => ({
            path: `services/billing-api/src/helper_${i}.ts`,
            lines: 60,
          })),
        ],
        dirty_file_count: 2,
        total_churn_lines: 75,
      },
      {
        name: 'legacy-archive',
        root: 'packages/legacy-archive',
        files: Array.from({ length: 10 }, (_, i) => ({
          path: `packages/legacy-archive/src/old_${i}.ts`,
          lines: 120,
        })),
        dirty_file_count: 0,
        total_churn_lines: 0,
      },
    ],
  };

  it('inflates active/dirty modules (2.0x–3.2x) and compacts untouched dormant modules (<= 0.65x)', () => {
    const activeMult = computeAttentionMultiplier(
      { activeAgentCount: 1, dirtyFileCount: 2, totalChurnLines: 75, errorCount: 1 },
      'attention',
    );
    const dormantMult = computeAttentionMultiplier(
      { activeAgentCount: 0, dirtyFileCount: 0, totalChurnLines: 0, errorCount: 0 },
      'attention',
    );
    const proportionalMult = computeAttentionMultiplier(
      { activeAgentCount: 2, dirtyFileCount: 5, errorCount: 3 },
      'proportional',
    );

    expect(activeMult).toBeGreaterThanOrEqual(2.0);
    expect(activeMult).toBeLessThanOrEqual(3.2);
    expect(dormantMult).toBeLessThanOrEqual(0.65);
    expect(proportionalMult).toBe(1.0);
  });

  it('gives an active/dirty module a significantly larger tile rect than an equal-file dormant module', () => {
    const layout = layoutTown(sampleScan, new Set(), {
      focusMode: 'attention',
      attentionByDistrict: {
        'billing-api': { activeAgentCount: 1, dirtyFileCount: 2, errorCount: 1 },
        'legacy-archive': { activeAgentCount: 0, dirtyFileCount: 0, errorCount: 0 },
      },
    });

    const billing = layout.districts.find((d) => d.name === 'billing-api')!;
    const legacy = layout.districts.find((d) => d.name === 'legacy-archive')!;

    const billingArea = (billing.x1 - billing.x0) * (billing.y1 - billing.y0);
    const legacyArea = (legacy.x1 - legacy.x0) * (legacy.y1 - legacy.y0);

    expect(billingArea).toBeGreaterThan(legacyArea * 1.5);
    expect(legacy.isCompactedDormant).toBe(true);
    expect(billing.isCompactedDormant).toBe(false);
  });

  it('allocates >= 65% of building plot footprint area to touched files inside an active district', () => {
    const layout = layoutTown(sampleScan, new Set(), { focusMode: 'attention' });
    const billing = layout.districts.find((d) => d.name === 'billing-api')!;

    let touchedFootprint = 0;
    let totalFootprint = 0;
    for (const b of billing.buildings) {
      const area = (b.colSpan ?? 1) * (b.rowSpan ?? 1);
      totalFootprint += area;
      if (b.isTouched) touchedFootprint += area;
    }

    expect(touchedFootprint / Math.max(1, totalFootprint)).toBeGreaterThanOrEqual(0.65);
  });
});
