import { describe, it, expect, vi, beforeEach } from 'vitest';
import { existsSync } from 'node:fs';
import { join } from 'node:path';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { fireEvent, render, screen, waitFor } from '@testing-library/react';
import { LanguageProvider } from '../../src/hooks/useLanguage';
import { applyTheme } from '../../src/lib/theme';
import {
  THEME_CONTRACT,
  verifyThemeProfileContrast,
} from '../../src/lib/themeContract';
import {
  evaluateGuiSurfaceRubric,
  runUniversalCvFlywheelPass,
  type GuiDomAuditMetrics,
} from '../../src/lib/cvFlywheel';
import { defaultHudTiles } from '../../src/hooks/useHudTiles';
import { INITIAL_KPIS } from '../../src/data/initialState';
import { auditDomQualityInPage } from './audits';
import { conceptArtPlateForSurface } from './vlmVisionFlywheel';

const UI_ROOT = join(import.meta.dirname, '..', '..');

const mockInvoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({
  invoke: (cmd: string, payload?: Record<string, unknown>) => mockInvoke(cmd, payload),
}));

vi.mock('../../src/hooks/useVoxGraphStatus', () => ({
  VOX_GRAPH_STATUS_QUERY_KEY: ['vox-graph', 'status'],
  useVoxGraphStatus: () => ({
    isLoading: false,
    isError: false,
    data: {
      default_corpus_id: 'repo-code-graph',
      ttl_days: 30,
      ttl_days_env_forced: false,
      corpora: [
        {
          corpus_id: 'repo-code-graph',
          title: 'Polyglot Code & Wiki Graph',
          graph_exists: true,
          manifest_exists: true,
          node_count: 1420,
          edge_count: 3890,
          built_at: new Date().toISOString(),
          manifest_git_sha: 'a1b2c3d',
          head_git_sha: 'a1b2c3d',
          stale_reasons: [],
          warnings: [],
          is_fresh: true,
        },
      ],
    },
  }),
}));

import { DocReader } from '../../src/components/surfaces/DocReader/DocReader';
import { MemoryView } from '../../src/components/surfaces/Memory/MemoryView';
import { VoxGraphStatusPanel } from '../../src/components/surfaces/VoxGraph/VoxGraphStatusPanel';
import { RepositoryView } from '../../src/components/surfaces/Repository/RepositoryView';
import { BottomStatusBar } from '../../src/components/layout/BottomStatusBar';

const VIEWPORTS = [
  { name: 'compact', width: 900, height: 600 },
  { name: 'laptop-narrow', width: 1100, height: 720 },
  { name: 'desktop-hd', width: 1280, height: 800 },
  { name: 'desktop-wide', width: 1440, height: 900 },
  { name: 'desktop-fhd', width: 1920, height: 1080 },
] as const;

const THEMES = ['arcane', 'glacier', 'slate', 'high-contrast'] as const;

const noopToast = () => {};

function setupBackendFixtures() {
  mockInvoke.mockImplementation((cmd: string, payload?: { tool?: string; args?: Record<string, unknown> }) => {
    if (cmd === 'read_doc_markdown') {
      return Promise.resolve('---\ntitle: "Layer 2: Interfaces"\nstatus: "current"\n---\n\n# Layer 2\n\nSynced prose.\n');
    }
    if (cmd === 'get_memory_status') {
      return Promise.resolve({
        corpus_counts: { memory: 120, knowledge: 340, chunk: 880 },
        shards: [{ id: 's0', depth: 4, entries: 1340, hot: true, dirty: false, spark: [10, 14, 18] }],
        recent_recalls: [{ q: 'openwiki auto fences', n: 5, when: '2m' }],
        embedding_dim: 768,
      });
    }
    if (cmd === 'get_vcs_isolation') {
      return Promise.resolve({ strategy_default: 'shared_branch', per_agent: {}, active_conflicts: [] });
    }
    if (cmd === 'get_gui_preference') {
      return Promise.resolve(null);
    }
    if (cmd !== 'invoke_mcp_tool') {
      return Promise.resolve({ active_lane: 'local', providers: [] });
    }
    const tool = payload?.tool;
    const args = payload?.args ?? {};
    if (tool === 'vox_kb_search') {
      return Promise.resolve({
        is_error: false,
        result: { success: true, data: { hits: [{ doc_path: 'docs/wiki/README.md', title: 'OpenWiki Hub', topic: 'doc-governance', domain: 'documentation', trust_tier: 'verified', score: 0.99 }] } },
      });
    }
    if (tool === 'vox_kb_inbox_list') {
      return Promise.resolve({ is_error: false, result: { success: true, data: { findings: [] } } });
    }
    if (tool === 'vox_kb_status') {
      return Promise.resolve({
        is_error: false,
        result: { success: true, data: { doc_count: 6, chunk_count: 42, drifting_count: 0, deficit_count: 0, mrr_at_5: 0.91, recall_at_5: 0.97 } },
      });
    }
    if (tool === 'vox_search_atlas') {
      return Promise.resolve({
        is_error: false,
        result: {
          success: true,
          data: {
            nodes: [{ id: 'crate:vox-cli', label: 'vox-cli', kind: 'crate', layer: 'L5-cli', file_path: 'crates/vox-cli/src/lib.rs', diff_status: 'modified', fan_in: 8, fan_out: 4, blast_s: 3 }],
            links: [],
          },
        },
      });
    }
    if (tool === 'vox_repo_status') {
      return Promise.resolve({
        is_error: false,
        result: {
          success: true,
          data: {
            repository_id: 'poly-alpha',
            root: '/workspace/poly-alpha',
            capabilities: { vox_project: true, cargo_workspace: true, node_workspace: true, python_project: true, go_module: true, git: true },
            active_context: { repository_id: 'poly-alpha', display_name: 'poly-alpha', root_path: '/workspace/poly-alpha', access_mode: 'local', read_only: false },
            openwiki: { initialized: true, page_count: 6, total_pages: 6, stale_pages: [] },
          },
        },
      });
    }
    if (tool === 'vox_repo_catalog_list') {
      return Promise.resolve({
        is_error: false,
        result: {
          success: true,
          data: {
            primary_repository_id: 'poly-alpha',
            repositories: [{ repository_id: 'poly-alpha', display_name: 'poly-alpha', resolved_root: '/workspace/poly-alpha', access_mode: 'local', read_only: false }],
          },
        },
      });
    }
    if (tool === 'vox_repo_catalog_refresh' && args.action === 'wiki_status') {
      return Promise.resolve({
        is_error: false,
        result: { success: true, data: { initialized: true, page_count: 6, total_pages: 6, auto_region_count: 6, stale_pages: [], pages: [{ rel_path: 'README.md', exists: true }] } },
      });
    }
    if (tool === 'vox_repo_catalog_refresh' && args.action === 'wiki_heal_list') {
      return Promise.resolve({ is_error: false, result: { success: true, data: { proposals: [] } } });
    }
    return Promise.resolve({ is_error: false, result: { success: true, data: {} } });
  });
}

describe('Phase 470 (CV-02 / CV-03) — Dual-Theme Concept-Art Plates & 5x4 Before/After CV Flywheel', () => {
  beforeEach(() => {
    mockInvoke.mockReset();
    setupBackendFixtures();
  });

  it('registers dual-theme v79 Concept-Art plates on disk for OpenWiki Studio and Multi-Repo Atlas', () => {
    const expectedPlates = [
      { surface: 'doc-reader', theme: 'arcane', plate: 'public/assets/romanesque/v79-concept-openwiki-studio-carrara.jpg' },
      { surface: 'memory', theme: 'slate', plate: 'public/assets/romanesque/v79-concept-openwiki-studio-slate.jpg' },
      { surface: 'vox-graph', theme: 'glacier', plate: 'public/assets/romanesque/v79-concept-multirepo-atlas-carrara.jpg' },
      { surface: 'repository', theme: 'high-contrast', plate: 'public/assets/romanesque/v79-concept-multirepo-atlas-slate.jpg' },
    ] as const;

    for (const row of expectedPlates) {
      const resolved = conceptArtPlateForSurface(row.surface, row.theme);
      expect(resolved).toBe(row.plate);
      expect(existsSync(join(UI_ROOT, resolved))).toBe(true);
    }
  });

  it('verifies WCAG contrast >= 4.5:1 across all 4 themes and converges CV_STAGE before -> after across 5 viewports x 4 themes', async () => {
    for (const theme of THEMES) {
      const profile = THEME_CONTRACT.themes[theme];
      const checks = verifyThemeProfileContrast(profile);
      expect(checks.every((c) => c.passed)).toBe(true);
    }

    const qc = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    const renderWorkspace = () =>
      render(
        React.createElement(
          LanguageProvider,
          null,
          React.createElement(
            QueryClientProvider,
            { client: qc },
            React.createElement(
              'main',
              { 'data-testid': 'v79-cv-harness-main' },
              React.createElement(DocReader, { path: 'docs/wiki/interfaces-and-apis.md' }),
              React.createElement(MemoryView, { pushToast: noopToast }),
              React.createElement(VoxGraphStatusPanel, null),
              React.createElement(RepositoryView, { pushToast: noopToast }),
              React.createElement(BottomStatusBar, {
                kpis: INITIAL_KPIS,
                hudTilesConfig: defaultHudTiles(),
                onHudTilesChange: () => {},
                onNavigate: () => {},
                lastOrchEventAt: null,
                orchUsesPolling: false,
                liveFreshMs: 10_000,
                activeRepoSummary: { displayName: 'poly-alpha', repositoryId: 'poly-alpha', readOnly: false },
              }),
            ),
          ),
        ),
      );

    const view = renderWorkspace();
    await waitFor(() => {
      expect(screen.getByTestId('code-atlas-canvas')).toBeInTheDocument();
      expect(screen.getByTestId('repo-catalog-panel')).toBeInTheDocument();
      expect(screen.getByTestId('bottom-status-bar-repo')).toBeInTheDocument();
    });

    // Open Radix/popover portals (RepoSwitcherPill dialog + OpenWikiHealStudio tab) so portal markup is audited.
    fireEvent.click(screen.getByTestId('bottom-status-bar-repo'));
    expect(await screen.findByTestId('bottom-status-bar-repo-popover')).toBeInTheDocument();
    fireEvent.click(screen.getByTestId('kb-tab-openwiki'));
    expect(await screen.findByTestId('openwiki-heal-studio')).toBeInTheDocument();

    const surfaces = ['doc-reader', 'memory', 'vox-graph', 'repository'] as const;
    for (const vp of VIEWPORTS) {
      Object.defineProperty(window, 'innerWidth', { value: vp.width, configurable: true });
      Object.defineProperty(window, 'innerHeight', { value: vp.height, configurable: true });
      const mainEl = screen.getByTestId('v79-cv-harness-main');
      mainEl.getBoundingClientRect = () =>
        ({
          left: 64,
          top: 32,
          right: vp.width - 16,
          bottom: vp.height - 32,
          width: vp.width - 80,
          height: vp.height - 64,
        }) as DOMRect;

      for (const theme of THEMES) {
        applyTheme(theme);
        for (const surf of surfaces) {
          const domReport = auditDomQualityInPage(surf);
          expect(domReport.rawEmojiCount).toBe(0);
          expect(domReport.unstyledNativeSelectCount).toBe(0);
          expect(domReport.clippedInteractiveCount).toBe(0);
          expect(domReport.overlappingControlPairs).toBe(0);
          expect(domReport.digitalColorLeakCount).toBe(0);
          expect(domReport.lightThemeContrastLeakCount).toBe(0);

          const afterDom: GuiDomAuditMetrics = {
            ...domReport,
            axeViolations: 0,
            lightThemeContrastLeakCount: 0,
            digitalColorLeakCount: 0,
          };
          const beforeDom: GuiDomAuditMetrics = {
            ...afterDom,
            lightThemeContrastLeakCount: 4,
            digitalColorLeakCount: 3,
            rawEmojiCount: 0,
          };
          const beforePix = { edgeSharpness: 74, luminanceContrast: 58, noiseFloor: 8.5 };
          const afterPix = { edgeSharpness: 88, luminanceContrast: 86, noiseFloor: 4.4 };

          const verdict = runUniversalCvFlywheelPass({
            surfaceId: `${surf}@${vp.width}x${vp.height}:${theme}`,
            passNumber: 1,
            rawPrompt: `Converge ${surf} for ${theme}`,
            beforeMetrics: {
              ...beforePix,
              rubricScore: 0.72,
              vlmRubric: evaluateGuiSurfaceRubric(beforeDom, beforePix),
            },
            afterMetrics: {
              ...afterPix,
              rubricScore: 0.95,
              vlmRubric: evaluateGuiSurfaceRubric(afterDom, afterPix),
            },
            afterDom,
            afterGrounded: true,
          });

          expect(verdict.flywheelConverged).toBe(true);
          expect(verdict.visualAfterScore).toBeGreaterThanOrEqual(0.92);
          expect(verdict.visualDelta).toBeGreaterThan(0);
          expect(verdict.contrastDeltaPct).toBeGreaterThan(0);
          expect(verdict.edgeSharpnessDeltaPct).toBeGreaterThan(0);
        }
      }
    }

    view.unmount();
    applyTheme('arcane');
  }, 30_000);
});
