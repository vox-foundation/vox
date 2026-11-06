// crates/vox-gui/ui/src/components/gamify/urbs/__tests__/worldRenderer.test.ts
import { describe, it, expect, vi } from 'vitest';
import {
  districtWonderKey,
  romanDistrictLabel,
  WorldRenderer,
  type WorldState,
} from '../worldRenderer';
import { layoutTown } from '../layout';
import type { TownScan } from '../types';

describe('worldRenderer (Phase 262: Interactive Foreground Worksite Cards & Repo-Agnostic Wonders)', () => {
  const polyglotScan: TownScan = {
    root: '/repo',
    scanned_at_ms: 0,
    truncated: false,
    repo_name: 'acme-cloud',
    primary_language: 'TypeScript',
    crates: [
      {
        name: 'shared-schema',
        root: 'packages/shared-schema',
        files: [
          {
            path: 'packages/shared-schema/src/user.ts',
            lines: 180,
            git_status: 'M',
            diff_added_lines: 34,
            diff_deleted_lines: 9,
          },
        ],
        deps: [],
        dirty_file_count: 1,
        total_churn_lines: 43,
      },
      {
        name: 'billing-service',
        root: 'services/billing-service',
        files: [
          {
            path: 'services/billing-service/src/charge.ts',
            lines: 420,
            git_status: 'M',
            diff_added_lines: 21,
            diff_deleted_lines: 5,
          },
        ],
        deps: ['shared-schema'],
        dirty_file_count: 1,
        total_churn_lines: 26,
      },
      {
        name: 'web-portal',
        root: 'apps/web-portal',
        files: [{ path: 'apps/web-portal/src/App.tsx', lines: 510 }],
        deps: ['billing-service'],
      },
    ],
  };

  it('derives wonder sprite keys and Roman district labels for non-Vox module names via structural archetypes', () => {
    expect(districtWonderKey('web-portal', false)).toBe('colosseum');
    expect(districtWonderKey('billing-service', false)).toBe('praetorium');
    expect(districtWonderKey('query-compiler', false)).toBe('landmark');
    expect(districtWonderKey('rag-embeddings', false)).toBe('rotunda');

    expect(romanDistrictLabel('billing-service')).toContain('PRAETORIVM');
    expect(romanDistrictLabel('web-portal')).toContain('AMPHITHEATRVM');
  });

  it('renders screen-space overlays with foreground worksite file diff slabs, blast radius, and dynamic error callouts', () => {
    const layout = layoutTown(polyglotScan, new Set(['billing-service']), {
      focusMode: 'attention',
    });
    const state: WorldState = {
      layout,
      buildings: {
        'services/billing-service/src/charge.ts': {
          x: 0,
          y: 0,
          warnings: 1,
          errors: 2,
        },
      },
      agentTasks: {
        'agent-1': {
          taskId: 't-1',
          filePath: 'packages/shared-schema/src/user.ts',
          status: 'Working',
        },
      },
      harness: { ci: null, vcs: null, queueLen: 2, mcp: null },
      selectedDistrict: 'shared-schema',
    };

    const fillTextCalls: string[] = [];
    const mockCtx = {
      save: vi.fn(),
      restore: vi.fn(),
      beginPath: vi.fn(),
      closePath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      roundRect: vi.fn(),
      rect: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      ellipse: vi.fn(),
      arc: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      measureText: vi.fn((text: string) => ({ width: text.length * 6 })),
      fillText: vi.fn((text: string) => {
        fillTextCalls.push(text);
      }),
      font: '',
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      textAlign: 'left',
      textBaseline: 'middle',
    } as unknown as CanvasRenderingContext2D;

    const renderer = new WorldRenderer();
    renderer.drawScreenSpaceOverlays(mockCtx, state, { x: 400, y: 300, zoom: 1 });

    // Verify blast radius badge on origin & downstream modules
    expect(fillTextCalls.some((t) => t.includes('Blast Radius: 2 downstream'))).toBe(true);
    expect(fillTextCalls.some((t) => t.includes('BLAST RADIUS'))).toBe(true);
    // Verify foreground file diff slab with git status + diff lines
    expect(fillTextCalls.some((t) => t.includes('M user.ts'))).toBe(true);
    expect(fillTextCalls.some((t) => t.includes('+34/-9'))).toBe(true);
    // Verify dynamic diagnostic error callout on billing-service
    expect(fillTextCalls.some((t) => t.includes('Fix 2 Diagnostic Errors'))).toBe(true);
  });
});



describe('worldRenderer (Phases 295–300 edge-softening integration)', () => {
  const scan: TownScan = {
    root: '/repo',
    scanned_at_ms: 0,
    truncated: false,
    repo_name: 'acme-cloud',
    primary_language: 'TypeScript',
    crates: [
      {
        name: 'shared-schema',
        root: 'packages/shared-schema',
        files: [{ path: 'packages/shared-schema/src/user.ts', lines: 180, git_status: 'M', diff_added_lines: 34, diff_deleted_lines: 9 }],
        deps: [],
        dirty_file_count: 1,
        total_churn_lines: 43,
      },
      {
        name: 'billing-service',
        root: 'services/billing-service',
        files: [{ path: 'services/billing-service/src/charge.ts', lines: 420 }],
        deps: ['shared-schema'],
      },
    ],
  };

  it('exposes WorldRenderer ensure/paint path without throwing on a polyglot layout', () => {
    const layout = layoutTown(scan, new Set(['billing-service']), { focusMode: 'attention' });
    const state: WorldState = {
      layout,
      buildings: {},
      agentTasks: {},
      harness: { ci: null, vcs: null, queueLen: 0, mcp: null },
      selectedDistrict: null,
    };
    const mockCtx = {
      canvas: { width: 800, height: 600 },
      save: vi.fn(),
      restore: vi.fn(),
      beginPath: vi.fn(),
      closePath: vi.fn(),
      moveTo: vi.fn(),
      lineTo: vi.fn(),
      quadraticCurveTo: vi.fn(),
      roundRect: vi.fn(),
      rect: vi.fn(),
      fillRect: vi.fn(),
      strokeRect: vi.fn(),
      clearRect: vi.fn(),
      ellipse: vi.fn(),
      arc: vi.fn(),
      fill: vi.fn(),
      stroke: vi.fn(),
      translate: vi.fn(),
      scale: vi.fn(),
      rotate: vi.fn(),
      createLinearGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      createRadialGradient: vi.fn(() => ({ addColorStop: vi.fn() })),
      createPattern: vi.fn(() => null),
      measureText: vi.fn((text: string) => ({ width: text.length * 6 })),
      fillText: vi.fn(),
      font: '',
      fillStyle: '',
      strokeStyle: '',
      lineWidth: 1,
      globalAlpha: 1,
      textAlign: 'left',
      textBaseline: 'middle',
      lineCap: 'butt',
      lineJoin: 'miter',
    } as unknown as CanvasRenderingContext2D;

    const renderer = new WorldRenderer();
    // jsdom lacks a real CanvasRenderingContext2D for offscreen ensure()/paint;
    // verify screen-space overlay path still runs against a mock context.
    expect(() => renderer.drawScreenSpaceOverlays(mockCtx, state, { x: 400, y: 300, zoom: 1 })).not.toThrow();
  });
});
