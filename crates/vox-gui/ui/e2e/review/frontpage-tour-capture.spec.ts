/**
 * Phase 533 — Up-to-date 16:10 (1440x900) Vox Axis GUI Tour Captures & CV Flywheel.
 *
 * Run:
 *   env VOX_TOUR_CAPTURE=1 pnpm --dir crates/vox-gui/ui exec playwright test e2e/review/frontpage-tour-capture.spec.ts --project=chromium
 */

import { test, expect, type Page } from '@playwright/test';
import AxeBuilder from '@axe-core/playwright';
import { mkdirSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { addRichMockInitScript } from '../lib/tauriMockRich';
import { auditDomQualityInPage, auditOverflowInPage } from './audits';
import { computeRgbaFrameCvMetrics } from '../../src/lib/cvFlywheel';

const RUN_TOUR = process.env.VOX_TOUR_CAPTURE === '1';
const OUT_DIR =
  process.env.VOX_TOUR_OUT_DIR ?? join(process.cwd(), 'test-results', 'tour-captures');

function installTourMockOverrides(): void {
  const internals = (window as any).__TAURI_INTERNALS__;
  if (!internals || typeof internals.invoke !== 'function') return;
  const prevInvoke: (cmd: string, args?: any) => Promise<unknown> = internals.invoke;

  const tourSearchResponse = {
    hits: [
      {
        source: 'chunk',
        kind: 'doc',
        path: 'docs/src/architecture/search-ssot.md',
        title: 'Hybrid RRF + HippoRAG Search SSOT',
        snippet:
          'RRF (k=60) fuses Tantivy BM25 lexical recall, HNSW dense vectors, and HippoRAG PPR.',
        score: 0.964,
        provenance: ['bm25', 'vector', 'ppr'],
        locator: { kind: 'file', value: 'docs/src/architecture/search-ssot.md' },
      },
      {
        source: 'repo',
        kind: 'code',
        path: 'crates/vox-search/src/rrf.rs',
        title: 'Reciprocal Rank Fusion Scorer',
        snippet:
          'fuse_ranked_lists(lists, k=60) merges multi-shard retrieval scores deterministically.',
        score: 0.921,
        provenance: ['ast', 'bm25'],
        locator: { kind: 'file', value: 'crates/vox-search/src/rrf.rs' },
      },
    ],
    facets_by_source: [
      { value: 'chunk', count: 1 },
      { value: 'repo', count: 1 },
    ],
    facets_by_kind: [
      { value: 'doc', count: 1 },
      { value: 'code', count: 1 },
    ],
    total: 2,
    next_cursor: null,
    corpora: ['memory', 'documentchunks', 'repoinventory'],
  };

  const tourResearchSessions = [
    {
      id: 1,
      status: 'completed',
      query_text: 'HippoRAG + Tantivy BM25 vs. Dense HNSW',
      started_at_ms: Date.now() - 3_600_000,
      finished_at_ms: Date.now() - 1_200_000,
    },
    {
      id: 2,
      status: 'completed',
      query_text: 'Socrates epistemic calibration across waves',
      started_at_ms: Date.now() - 7_200_000,
      finished_at_ms: Date.now() - 5_400_000,
    },
    {
      id: 3,
      status: 'completed',
      query_text: 'Zero-stub arXiv LaTeX + BibTeX verification',
      started_at_ms: Date.now() - 10_800_000,
      finished_at_ms: Date.now() - 9_000_000,
    },
    {
      id: 4,
      status: 'completed',
      query_text: 'OpenWiki Tier-1 AST anchor repair determinism',
      started_at_ms: Date.now() - 14_400_000,
      finished_at_ms: Date.now() - 12_600_000,
    },
  ];

  const tourResearchDetail = {
    session: tourResearchSessions[0],
    report_markdown: [
      'Fusing **Tantivy BM25** exact token recall with **HNSW dense vectors** and **HippoRAG Personalized PageRank (PPR)** via RRF (`k=60`) outperforms single-retriever pipelines:',
      '',
      '- **Multi-Hop Accuracy:** Graph-guided PPR boosts cross-file retrieval (`+18.4%` MRR@10) with zero extra LLM turns.',
      '- **Sub-12ms Lexical Gating:** Block-WAND pruning eliminates missed symbol tokens in pure cosine search.',
      '- **3-Layer Isolation:** `KbScopeMask` enforces deterministic separation across `Repo-Local`, `Global Library`, and `Vox Canon`.',
    ].join('\n'),
    artifact_json: '{}',
    confidence_tier: 'DeepResearch',
    source_count: 6,
    citation_precision: 0.98,
    wave_count: 3,
    wave_stability: 0.96,
    quality_score: 0.96,
    synthesis_model: 'anthropic/claude-sonnet',
    claims: [
      {
        claim_id: 'c1',
        text: 'BM25 block-WAND recovers exact .vox symbol tokens missed by cosine search.',
        verdict: 'Supported',
        confidence: 0.97,
        resample_stability: 0.95,
        citation_urls: [
          'https://arxiv.org/abs/2405.14831',
          'https://github.com/vox-foundation/vox/blob/main/docs/src/reference/ref-search.md',
        ],
        corroboration_count: 3,
      },
      {
        claim_id: 'c2',
        text: 'RRF + HippoRAG PPR lifts multi-hop recall by +18.4% over dense HNSW alone.',
        verdict: 'Supported',
        confidence: 0.95,
        resample_stability: 0.93,
        citation_urls: [
          'https://arxiv.org/abs/2405.14831',
          'https://dl.acm.org/doi/10.1145/1571941.1572114',
        ],
        corroboration_count: 3,
      },
      {
        claim_id: 'c3',
        text: 'KbScopeMask + BLAKE3 hashing isolate multi-repo shards cleanly.',
        verdict: 'Supported',
        confidence: 0.94,
        resample_stability: 0.92,
        citation_urls: [
          'https://github.com/vox-foundation/vox/blob/main/docs/src/architecture/federated-kb-openwiki-2026.md',
        ],
        corroboration_count: 2,
      },
    ],
  };

  const tourManifests = [
    {
      publication_id: 'pub-1',
      title: 'Hybrid RRF + PPR',
      content_type: 'paper',
      candidate_class: 'retrieval',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: false,
      state: 'draft',
      created_at_ms: Date.now() - 86_400_000 * 6,
      updated_at_ms: Date.now() - 3600_000 * 6,
    },
    {
      publication_id: 'pub-2',
      title: 'WebIR Codegen',
      content_type: 'paper',
      candidate_class: 'compiler',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: false,
      state: 'draft',
      created_at_ms: Date.now() - 86_400_000 * 5,
      updated_at_ms: Date.now() - 3600_000 * 5,
    },
    {
      publication_id: 'pub-3',
      title: 'ORCID Ledger',
      content_type: 'paper',
      candidate_class: 'provenance',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: false,
      state: 'doi_reserved',
      created_at_ms: Date.now() - 86_400_000 * 4,
      updated_at_ms: Date.now() - 3600_000 * 4,
    },
    {
      publication_id: 'pub-4',
      title: 'SHA3 IMRaD Gate',
      content_type: 'paper',
      candidate_class: 'repro_gate',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'orcid_attributed',
      created_at_ms: Date.now() - 86_400_000 * 4,
      updated_at_ms: Date.now() - 3600_000 * 3,
    },
    {
      publication_id: 'pub-5',
      title: 'Candle QLoRA',
      content_type: 'paper',
      candidate_class: 'mens_ml',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'approved',
      created_at_ms: Date.now() - 86_400_000 * 3,
      updated_at_ms: Date.now() - 3600_000 * 3,
    },
    {
      publication_id: 'pub-6',
      title: 'Actor Replay',
      content_type: 'paper',
      candidate_class: 'durability',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'approved',
      created_at_ms: Date.now() - 86_400_000 * 3,
      updated_at_ms: Date.now() - 3600_000 * 2,
    },
    {
      publication_id: 'pub-7',
      title: 'OpenWiki Heal',
      content_type: 'paper',
      candidate_class: 'governance',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'submitted',
      created_at_ms: Date.now() - 86_400_000 * 2,
      updated_at_ms: Date.now() - 3600_000 * 2,
    },
    {
      publication_id: 'pub-8',
      title: 'Socrates Eval',
      content_type: 'paper',
      candidate_class: 'calibration',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'published',
      created_at_ms: Date.now() - 86_400_000 * 2,
      updated_at_ms: Date.now() - 3600_000,
    },
    {
      publication_id: 'pub-9',
      title: 'Vox vs TS Pass@1',
      content_type: 'paper',
      candidate_class: 'empirical',
      publication_tier: 'arxiv_preprint',
      arxiv_ready: true,
      state: 'published',
      created_at_ms: Date.now() - 86_400_000,
      updated_at_ms: Date.now() - 1800_000,
    },
    {
      publication_id: 'pub-10',
      title: 'Replay Diff Audit',
      content_type: 'paper',
      candidate_class: 'telemetry',
      publication_tier: 'tech_report',
      arxiv_ready: false,
      state: 'failed',
      created_at_ms: Date.now() - 86_400_000,
      updated_at_ms: Date.now() - 900_000,
    },
  ];

  internals.invoke = async (cmd: string, args?: any) => {
    switch (cmd) {
      case 'get_routing_summary_live':
        return {
          active_model: 'anthropic/claude-sonnet',
          exploration_spent_usd: 2.4,
          exploration_budget_usd: 50,
          arm_count: 6,
          model_count: 32,
          family: 'anthropic/claude-sonnet',
          resolved_from: 'catalog',
          reason: 'lowest cost that fits the mode',
          decision_preview: {
            selected_model: 'anthropic/claude-sonnet',
            discovery_state: 'confirmed',
            alternatives: ['google/gemini-pro', 'mens/vox-qwen3-8b'],
            rejection_reasons: [],
            intelligence_score: 0.94,
            efficiency_score: 0.85,
            latency_score: 0.88,
          },
        };
      case 'get_memory_status':
        return {
          embedding_dim: 768,
          corpus_counts: { proj: 1280, docs: 540, chats: 96, rules: 210, web: 60 },
          recent_recalls: [
            { q: 'HippoRAG PageRank fusion', n: 4, when: '2m' },
            { q: 'ed25519 signature envelope', n: 6, when: '14m' },
            { q: 'OpenWiki Tier-1 drift heal', n: 5, when: '38m' },
          ],
          shards: [
            { id: 'proj', depth: 3, entries: 1280, hot: true, dirty: false, spark: [2, 5, 3, 8, 6, 9, 7] },
            { id: 'docs', depth: 2, entries: 540, hot: false, dirty: true, spark: [1, 2, 1, 3, 2, 4, 3] },
            { id: 'chats', depth: 1, entries: 96, hot: false, dirty: false, spark: [0, 1, 0, 2, 1, 1, 2] },
            { id: 'rules', depth: 2, entries: 210, hot: true, dirty: false, spark: [1, 1, 2, 2, 3, 2, 3] },
          ],
        };
      case 'vox_search_query':
        return tourSearchResponse;
      case 'list_research_sessions':
        return tourResearchSessions;
      case 'get_research_session_detail':
        return tourResearchDetail;
      case 'list_publication_manifests':
        return tourManifests;
      case 'list_orchestrator_tasks':
        return [
          {
            id: 200,
            description: 'WebIR Typecheck',
            priority: 'high',
            lifecycle: 'done',
            agent_id: 1,
            session_id: 'session-1',
            estimated_complexity: 2,
            depends_on: [],
            write_files: ['crates/vox-compiler/src/hir/lower.rs'],
            remote_node: null,
            origin: 'orchestrator',
          },
          {
            id: 201,
            description: 'Federated RRF',
            priority: 'high',
            lifecycle: 'in_progress',
            agent_id: 2,
            session_id: 'session-1',
            estimated_complexity: 2,
            depends_on: [200],
            write_files: ['crates/vox-search/src/rrf.rs'],
            remote_node: null,
            origin: 'orchestrator',
          },
          {
            id: 202,
            description: 'Clavis Vault',
            priority: 'normal',
            lifecycle: 'pending',
            agent_id: 3,
            session_id: 'session-1',
            estimated_complexity: 1,
            depends_on: [201],
            write_files: ['crates/vox-secrets/src/resolver.rs'],
            remote_node: null,
            origin: 'orchestrator',
          },
          {
            id: 203,
            description: 'Doctest Suite',
            priority: 'normal',
            lifecycle: 'pending',
            agent_id: 4,
            session_id: 'session-1',
            estimated_complexity: 1,
            depends_on: [202],
            write_files: ['crates/vox-doc-pipeline/src/pipeline/lint.rs'],
            remote_node: null,
            origin: 'orchestrator',
          },
        ];
      case 'workspace_town_scan': {
        const res = (await prevInvoke(cmd, args)) as { crates?: Array<Record<string, unknown>> };
        if (res && Array.isArray(res.crates)) {
          res.crates = res.crates.map((c) => {
            if (c.name === 'vox-compiler' && Array.isArray(c.files)) {
              return { ...c, files: c.files.slice(0, 4) };
            }
            return c;
          });
        }
        return res;
      }
      case 'chat_get_messages':
        return [
          {
            id: 1,
            role: 'user',
            content: 'Verify hybrid BM25 + HippoRAG recall across crates/vox-search and emit a typed .vox query.',
            created_at: 'now',
            task_id: null,
          },
          {
            id: 2,
            role: 'assistant',
            content: [
              'Audited `crates/vox-search/src/rrf.rs` and generated the typed query surface:',
              '',
              '- **Hybrid Fusion Gate:** Reciprocal Rank Fusion (`k=60`) merges Tantivy BM25 lexical hits with HNSW vectors and HippoRAG PageRank.',
              '- **Session Inspector:** Linked live BPE cache telemetry (`83.4%` K-prefix hit rate) and `HMAC` dispatch receipts.',
              '',
              '```vox',
              'query recall_kb(q: str, top_k: int) to Result[list[KbHit]] {',
              '    return vox_search.rrf_query(q, top_k, k=60)',
              '}',
              '```',
            ].join('\n'),
            created_at: 'now',
            task_id: '201',
            model_id: 'anthropic/claude-sonnet',
            latency_ms: 1420,
            selection_reason: 'Spoke: SearchRetrieval · Precision 0.96',
            events: [
              { id: 'ev-1', ts: 1, kind: 'thinking', summary: 'Thought for 1.4s — verified rrf.rs and K-prefix cache' },
              { id: 'ev-2', ts: 2, kind: 'tool_start', tool: 'vox_read_slice', summary: 'crates/vox-search/src/rrf.rs' },
              { id: 'ev-3', ts: 3, kind: 'tool_end', tool: 'vox_read_slice', ok: true, summary: 'Read 120 lines (RRF k=60 scorer)' },
            ],
          },
        ];
      case 'read_doc_markdown':
        return [
          '---',
          'title: "Hybrid Retrieval & OpenWiki Guide"',
          'category: "explanation"',
          'status: "current"',
          'domain: "knowledge"',
          '---',
          '',
          '# Hybrid Retrieval & OpenWiki Guide',
          '',
          '<!-- kb:summary -->',
          'Vox fuses Tantivy BM25 lexical matching, HNSW vector shards, and HippoRAG PageRank over a 3-layer Federated KB.',
          '<!-- /kb:summary -->',
          '',
          '> [!NOTE]',
          '> Non-modal Doc Dock keeps Research, Memory, and Chat live side by side.',
          '',
          '| KB Layer | Scope | Path |',
          '|---|---|---|',
          '| Vox Baseline | Canon docs | `docs/src/` |',
          '| Repo-Local | Project OpenWiki | `docs/wiki/` |',
        ].join('\n');
      case 'chat_list_sessions':
        return [
          { session_id: 'session-1', title: '1 · Hybrid RRF Recall', message_count: 2, updated_at: '2026-11-19T08:04:00Z', conversation_id: 1, repository_id: 'vox' },
          { session_id: 'session-2', title: '2 · WebIR Typecheck', message_count: 4, updated_at: '2026-11-19T08:03:00Z', conversation_id: 2, repository_id: 'vox' },
          { session_id: 'session-3', title: '3 · OpenWiki Self-Heal', message_count: 6, updated_at: '2026-11-19T08:02:00Z', conversation_id: 3, repository_id: 'vox' },
          { session_id: 'session-4', title: '4 · Scientia arXiv Gate', message_count: 3, updated_at: '2026-11-19T08:01:00Z', conversation_id: 4, repository_id: 'vox' },
        ];
      case 'activity_query': {
        const res = (await prevInvoke(cmd, args)) as Array<Record<string, unknown>>;
        return Array.isArray(res)
          ? res.map((row) => ({
              ...row,
              summary:
                typeof row.summary === 'string'
                  ? row.summary.replace('32-surface', '33-surface')
                  : row.summary,
            }))
          : res;
      }
      case 'scientia_dashboard_snapshot': {
        const res = (await prevInvoke(cmd, args)) as any;
        if (res && res.candidates && Array.isArray(res.candidates.top_5_by_confidence)) {
          return {
            ...res,
            candidates: {
              ...res.candidates,
              top_5_by_confidence: res.candidates.top_5_by_confidence.slice(0, 2),
            },
          };
        }
        return res;
      }
      default:
        return prevInvoke(cmd, args);
    }
  };
}

async function emitLivePulse(page: Page) {
  await page.evaluate(() => {
    const emit = (window as any).__TAURI_EMIT__;
    if (typeof emit !== 'function') return;
    const now = Date.now();
    emit('vox://orch-status', {
      agent_count: 9,
      total_queued: 44,
      total_in_progress: 6,
      total_completed: 128,
      total_doubted: 1,
      total_weighted_load: 7.5,
      predicted_load: 8.2,
      agents: Array.from({ length: 9 }, (_, i) => ({
        id: i + 1,
        name: `agent-${i + 1}`,
        in_progress: i % 3 !== 0,
        paused: false,
        progress: 0.65,
        current_phase: 'verify',
        task_description: 'Federated KB + Compiler verification',
        cost: 0.42,
        budget: 5,
      })),
      recent_events: [
        {
          id: now,
          kind: 'phase_change',
          tag: 'agent-1',
          title: 'verified',
          body: 'ok',
          timestamp: String(now),
        },
      ],
      alerts: [],
      peers: [
        { id: 'node-a', status: 'online' },
        { id: 'node-b', status: 'online' },
      ],
      total_cost: 12.34,
      budget_cap: 50,
      mesh_throughput: 3.2,
    });
  });
  await page.waitForTimeout(180);
}

async function dismissAllToasts(page: Page) {
  await page.evaluate(() => {
    const btns = Array.from(document.querySelectorAll('[data-testid="toast-stack"] button'));
    for (const b of btns) {
      if (b instanceof HTMLElement) b.click();
    }
  });
  await page.waitForTimeout(260);
}

async function decodePngRgbaInPage(page: Page, pngBuffer: Buffer) {
  const b64 = pngBuffer.toString('base64');
  const raw = await page.evaluate(async (enc) => {
    const res = await fetch(`data:image/png;base64,${enc}`);
    const bmp = await createImageBitmap(await res.blob());
    try {
      const w = Math.max(8, Math.floor(bmp.width * 0.25));
      const h = Math.max(8, Math.floor(bmp.height * 0.25));
      const canvas = document.createElement('canvas');
      canvas.width = w;
      canvas.height = h;
      const ctx = canvas.getContext('2d');
      if (!ctx) return { w: 0, h: 0, bytes: [] as number[] };
      ctx.drawImage(bmp, 0, 0, w, h);
      return { w, h, bytes: Array.from(ctx.getImageData(0, 0, w, h).data) };
    } finally {
      bmp.close();
    }
  }, b64);
  return computeRgbaFrameCvMetrics(new Uint8Array(raw.bytes), raw.w, raw.h);
}

async function auditCapture(page: Page, id: string) {
  await emitLivePulse(page);
  const path = join(OUT_DIR, `${id}.png`);
  const buf = await page.screenshot({ path, animations: 'disabled' });
  const pixelSignals = await decodePngRgbaInPage(page, buf);
  let axeViolations = 0;
  let axeDetails: Array<{ id: string; impact: string | null | undefined; targets: unknown[] }> = [];
  try {
    const axe = await new AxeBuilder({ page }).options({ resultTypes: ['violations'] }).analyze();
    const serious = axe.violations.filter((v) =>
      ['serious', 'critical'].includes(v.impact ?? ''),
    );
    axeViolations = serious.length;
    axeDetails = serious.map((v) => ({
      id: v.id,
      impact: v.impact,
      targets: v.nodes.map((n) => n.target),
    }));
  } catch (err) {
    axeViolations = -1;
    axeDetails = [{ id: 'axe-runtime-error', impact: 'critical', nodes: 1 }];
    void err;
  }
  const domQuality = await page.evaluate(auditDomQualityInPage, id);
  const overflow = await page.evaluate(auditOverflowInPage);
  return {
    id,
    path,
    pixelSignals,
    axeViolations,
    axeDetails,
    outerOverflowPx: Math.max(domQuality.outerOverflowPx, overflow.bodyHorizontalOverflowPx),
    clippedInteractiveCount: domQuality.clippedInteractiveCount,
    rawBacktickCount: domQuality.rawBacktickCount,
    lightThemeContrastLeakCount: domQuality.lightThemeContrastLeakCount,
  };
}

test.describe('Phase 533 — Fresh 16:10 Frontpage GUI Tour Captures & CV Flywheel', () => {
  test.skip(!RUN_TOUR, 'set VOX_TOUR_CAPTURE=1 to capture fresh GUI tour screenshots');

  test('captures glitch-free 1440x900 Vox Axis showcase frames across Light (arcane) and Dark (slate)', async ({
    page,
  }) => {
    test.setTimeout(240_000);
    mkdirSync(OUT_DIR, { recursive: true });
    await page.setViewportSize({ width: 1440, height: 900 });
    await addRichMockInitScript(page, 'gamify');
    await page.addInitScript(`(${installTourMockOverrides.toString()})();`);

    await page.goto('/#view=gamify');
    await page.waitForSelector('nav', { timeout: 20_000 });
    await page.evaluate(async () => {
      document.documentElement.setAttribute('data-theme', 'arcane');
      window.localStorage.setItem('vox-theme', 'arcane');
      await (document as any).fonts?.ready;
    });

    const results: Record<string, Awaited<ReturnType<typeof auditCapture>>> = {};

    // 1. urbs-civilization
    const urbsTab = page.getByTestId('ludus-subtab-urbs');
    if (await urbsTab.isVisible().catch(() => false)) {
      await urbsTab.click();
    }
    await page.waitForSelector('[data-testid="urbs-minimap-radar"]', { timeout: 10_000 });
    await page.waitForTimeout(600);
    await page.mouse.move(0, 0);
    results['urbs-civilization'] = await auditCapture(page, 'urbs-civilization');

    // 2. urbs-imperium-inspector
    const forvmBtn = page.locator('[data-urbs-landmark="FORVM"]').first();
    await forvmBtn.click();
    await expect(page.getByTestId('urbs-imperium-inspector')).toBeVisible();
    await page.waitForTimeout(300);
    await page.mouse.move(0, 0);
    results['urbs-imperium-inspector'] = await auditCapture(page, 'urbs-imperium-inspector');

    // 3. chat-multidock
    await page.evaluate(() => {
      window.location.hash = '#view=chat';
    });
    await page.waitForSelector('[data-testid="loquela-composer"]', { timeout: 10_000 });
    const multiBtn = page.getByTestId('chat-header-preset-multi');
    if (await multiBtn.isVisible().catch(() => false)) {
      await multiBtn.click();
      await page.waitForTimeout(450);
    }
    const execTabBtn = page.locator('[role="tab"], button').filter({ hasText: /^Execution$/i }).first();
    if (await execTabBtn.isVisible().catch(() => false)) {
      await execTabBtn.click();
      await page.waitForTimeout(250);
    }
    const composerArea = page.locator('#loquela-composer').first();
    if (await composerArea.isVisible().catch(() => false)) {
      await composerArea.fill('Verify hybrid RRF recall in crates/vox-search');
    }
    await page.mouse.move(0, 0);
    results['chat-multidock'] = await auditCapture(page, 'chat-multidock');

    // 4. harness
    await page.evaluate(() => {
      window.location.hash = '#view=harness';
    });
    await page.waitForSelector('[data-testid="harness-agent-braid"]', { timeout: 10_000 });
    await page.waitForTimeout(450);
    await page.mouse.move(0, 0);
    results['harness'] = await auditCapture(page, 'harness');

    // 5. research-arcane (+ research-slate)
    await page.evaluate(() => {
      window.location.hash = '#view=research';
    });
    await page.waitForSelector('[data-testid="research-session-1"], button', { timeout: 10_000 });
    const sessionBtn = page
      .locator('button')
      .filter({ hasText: /HippoRAG \+ Tantivy/i })
      .first();
    if (await sessionBtn.isVisible().catch(() => false)) {
      await sessionBtn.click();
      await page.waitForTimeout(350);
    }
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['research-arcane'] = await auditCapture(page, 'research-arcane');

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'void');
      window.localStorage.setItem('vox-theme', 'void');
    });
    await page.waitForTimeout(200);
    results['research-slate'] = await auditCapture(page, 'research-slate');

    // Switch back to arcane for memory, doc-reader, publications
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'arcane');
      window.localStorage.setItem('vox-theme', 'arcane');
    });

    // 6. memory-arcane (+ memory-slate)
    await page.evaluate(() => {
      window.location.hash = '#view=memory';
    });
    const recallInput = page.getByLabel('Recall query');
    await recallInput.waitFor({ state: 'visible', timeout: 10_000 });
    await recallInput.fill('HippoRAG PageRank fusion');
    await recallInput.press('Enter');
    await page.waitForTimeout(450);
    await dismissAllToasts(page);
    const shardsSummary = page.locator('details[open] summary', { hasText: /Mounted Vector Shards/i }).first();
    if (await shardsSummary.isVisible().catch(() => false)) {
      await shardsSummary.click();
      await page.waitForTimeout(120);
    }
    await page.mouse.move(0, 0);
    results['memory-arcane'] = await auditCapture(page, 'memory-arcane');

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'void');
      window.localStorage.setItem('vox-theme', 'void');
    });
    await page.waitForTimeout(200);
    results['memory-slate'] = await auditCapture(page, 'memory-slate');

    // Switch back to arcane for doc-reader & publications
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'arcane');
      window.localStorage.setItem('vox-theme', 'arcane');
    });
    await page.waitForTimeout(200);

    // 7. doc-reader-arcane (+ doc-reader-slate) via non-modal vox://open-doc beside live Memory
    await page.evaluate(() => {
      window.dispatchEvent(
        new CustomEvent('vox://open-doc', {
          detail: { path: 'docs/wiki/hybrid-retrieval-guide.md', initialMode: 'read' },
        }),
      );
    });
    await page.waitForSelector('[data-testid="doc-reader"]', { timeout: 10_000 });
    await page.waitForTimeout(350);
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['doc-reader-arcane'] = await auditCapture(page, 'doc-reader-arcane');

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'void');
      window.localStorage.setItem('vox-theme', 'void');
    });
    await page.waitForTimeout(200);
    results['doc-reader-slate'] = await auditCapture(page, 'doc-reader-slate');

    // Close doc dock cleanly before publications
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'arcane');
      window.localStorage.setItem('vox-theme', 'arcane');
    });
    const closeDockBtn = page.getByRole('button', { name: 'Close doc' }).first();
    if (await closeDockBtn.isVisible().catch(() => false)) {
      await closeDockBtn.click();
      await page.waitForTimeout(200);
    }

    // 8. publications-arcane
    await page.evaluate(() => {
      window.location.hash = '#view=publications';
    });
    await page.waitForSelector('[data-testid="publications-pipeline-overview"]', { timeout: 10_000 }).catch(async () => {
      await page.waitForTimeout(500);
    });
    await page.waitForTimeout(450);
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['publications-arcane'] = await auditCapture(page, 'publications-arcane');

    // 9. theme-arcane & 10. theme-slate (Settings -> Theme)
    await page.evaluate(() => {
      window.location.hash = '#view=settings';
    });
    await page.waitForTimeout(350);
    const themeNavBtn = page.getByRole('button', { name: /^Theme$/i }).first();
    if (await themeNavBtn.isVisible().catch(() => false)) {
      await themeNavBtn.click();
      await page.waitForTimeout(250);
    }
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['theme-arcane'] = await auditCapture(page, 'theme-arcane');

    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'void');
      window.localStorage.setItem('vox-theme', 'void');
    });
    await page.waitForTimeout(250);
    results['theme-slate'] = await auditCapture(page, 'theme-slate');

    // Restore Imperial Carrara (arcane) for tutorial knowledge captures
    await page.evaluate(() => {
      document.documentElement.setAttribute('data-theme', 'arcane');
      window.localStorage.setItem('vox-theme', 'arcane');
    });
    await page.waitForTimeout(200);

    // 11. vox-search-arcane (Knowledge -> Search Index)
    await page.evaluate(() => {
      window.location.hash = '#view=vox-search';
    });
    await page.waitForSelector('text=/SEARCH INDEX HEALTH/i', { timeout: 10_000 }).catch(async () => {
      await page.waitForTimeout(500);
    });
    await page.waitForTimeout(450);
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['vox-search-arcane'] = await auditCapture(page, 'vox-search-arcane');

    // 12. discovery-arcane (Knowledge -> Discovery / Activity)
    await page.evaluate(() => {
      window.location.hash = '#view=activity';
    });
    await page.waitForSelector('text=/EPIS[A-Z ]*DISCOVERY/i', { timeout: 10_000 }).catch(async () => {
      await page.waitForTimeout(500);
    });
    await page.waitForTimeout(450);
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['discovery-arcane'] = await auditCapture(page, 'discovery-arcane');

    // 13. scientia-arcane (Knowledge -> Findings / Scientia)
    await page.evaluate(() => {
      window.location.hash = '#view=scientia';
    });
    await page.waitForSelector('text=/SCIENTIA EPISTEMIC/i', { timeout: 10_000 }).catch(async () => {
      await page.waitForTimeout(500);
    });
    await page.waitForTimeout(450);
    await dismissAllToasts(page);
    await page.mouse.move(0, 0);
    results['scientia-arcane'] = await auditCapture(page, 'scientia-arcane');

    writeFileSync(join(OUT_DIR, 'cv-report.json'), JSON.stringify(results, null, 2));

    for (const [id, rec] of Object.entries(results)) {
      expect(
        rec.axeViolations,
        `${id} axeViolations: ${JSON.stringify(rec.axeDetails)}`,
      ).toBe(0);
      expect(rec.outerOverflowPx, `${id} outerOverflowPx`).toBe(0);
    }
  });
});
