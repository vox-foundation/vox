// crates/vox-gui/ui/src/components/gamify/urbs/repoPresets.ts
import type { TownScan } from './types';

export type UrbsRepoPresetId = 'live' | 'ts_monorepo' | 'python_ml' | 'go_microservices';

export interface UrbsRepoPreset {
  id: UrbsRepoPresetId;
  label: string;
  language: string;
  scan: TownScan;
  diagnostics: Record<string, { warnings: number; errors: number }>;
  activeTasks: Record<
    string,
    { taskId: string; agentId: string; agentRole: string; filePath: string; status: string }
  >;
  needsYouGates: Array<{ id: string; district: string; title: string }>;
}

/**
 * Default live fallback scan when Tauri backend is offline (e.g. browser/Playwright/unit test mode).
 * Uses repo-agnostic polyglot module names with rich dependency topology and git diff signals.
 */
export const DEFAULT_LIVE_FALLBACK_SCAN: TownScan = {
  root: '/workspace/polyglot-engine',
  scanned_at_ms: 1729360000000,
  truncated: false,
  repo_name: 'polyglot-engine',
  primary_language: 'Rust',
  crates: [
    {
      name: 'core-compiler',
      root: 'crates/core-compiler',
      deps: ['storage-db'],
      dirty_file_count: 2,
      total_churn_lines: 68,
      files: [
        { path: 'crates/core-compiler/src/lib.rs', lines: 5100 },
        {
          path: 'crates/core-compiler/src/parser.rs',
          lines: 6800,
          git_status: 'M',
          diff_added_lines: 44,
          diff_deleted_lines: 12,
        },
        {
          path: 'crates/core-compiler/src/typeck/mod.rs',
          lines: 6200,
          git_status: 'M',
          diff_added_lines: 9,
          diff_deleted_lines: 3,
        },
        { path: 'crates/core-compiler/src/codegen/rust.rs', lines: 6400 },
        { path: 'crates/core-compiler/src/codegen/ts.rs', lines: 4900 },
        { path: 'crates/core-compiler/src/hir/lower.rs', lines: 4400 },
        { path: 'crates/core-compiler/src/lexer/cursor.rs', lines: 2900 },
        { path: 'crates/core-compiler/src/ast/decl.rs', lines: 3200 },
        { path: 'crates/core-compiler/src/mir/pass.rs', lines: 2100 },
      ],
    },
    {
      name: 'agent-orchestrator',
      root: 'crates/agent-orchestrator',
      deps: ['core-compiler', 'storage-db', 'ml-scholar'],
      dirty_file_count: 1,
      total_churn_lines: 39,
      files: [
        { path: 'crates/agent-orchestrator/src/lib.rs', lines: 4800 },
        {
          path: 'crates/agent-orchestrator/src/models/registry.rs',
          lines: 6400,
          git_status: 'M',
          diff_added_lines: 31,
          diff_deleted_lines: 8,
        },
        { path: 'crates/agent-orchestrator/src/models/select.rs', lines: 5600 },
        { path: 'crates/agent-orchestrator/src/mcp_tools/agent_loop.rs', lines: 7200 },
        { path: 'crates/agent-orchestrator/src/mcp_tools/chat_tools.rs', lines: 5200 },
        { path: 'crates/agent-orchestrator/src/compaction/engine.rs', lines: 4500 },
        { path: 'crates/agent-orchestrator/src/hopper/queue.rs', lines: 4300 },
      ],
    },
    {
      name: 'studio-gui',
      root: 'crates/studio-gui',
      deps: ['core-compiler', 'agent-orchestrator'],
      dirty_file_count: 1,
      total_churn_lines: 52,
      files: [
        { path: 'crates/studio-gui/src/main.rs', lines: 3200 },
        { path: 'crates/studio-gui/src/commands/chat_turn.rs', lines: 4600 },
        {
          path: 'crates/studio-gui/src/commands/workspace_town.rs',
          lines: 3400,
          git_status: 'M',
          diff_added_lines: 41,
          diff_deleted_lines: 11,
        },
        { path: 'crates/studio-gui/ui/src/App.tsx', lines: 6800 },
        { path: 'crates/studio-gui/ui/src/components/surfaces/Chat/ChatSurface.tsx', lines: 5200 },
        { path: 'crates/studio-gui/ui/src/components/gamify/LudusSandbox.tsx', lines: 4800 },
      ],
    },
    {
      name: 'storage-db',
      root: 'crates/storage-db',
      deps: ['crypto-vault'],
      files: [
        { path: 'crates/storage-db/src/lib.rs', lines: 2900 },
        { path: 'crates/storage-db/src/schema/migrations.rs', lines: 3700 },
        { path: 'crates/storage-db/src/telemetry/research.rs', lines: 2800 },
        { path: 'crates/storage-db/src/cas/store.rs', lines: 2400 },
        { path: 'crates/storage-db/src/sessions/context.rs', lines: 2200 },
      ],
    },
    {
      name: 'ml-scholar',
      root: 'crates/ml-scholar',
      deps: ['storage-db', 'crypto-vault'],
      files: [
        { path: 'crates/ml-scholar/src/lib.rs', lines: 2100 },
        { path: 'crates/ml-scholar/src/qlora/trainer.rs', lines: 3400 },
        { path: 'crates/ml-scholar/src/eval/spoke_scorecard.rs', lines: 2600 },
        { path: 'crates/ml-scholar/src/dpo/flywheel.rs', lines: 2100 },
        { path: 'crates/ml-scholar/src/serve/engine.rs', lines: 1800 },
      ],
    },
    {
      name: 'crypto-vault',
      root: 'crates/crypto-vault',
      files: [
        { path: 'crates/crypto-vault/src/lib.rs', lines: 680 },
        { path: 'crates/crypto-vault/src/aead/chacha.rs', lines: 720 },
        { path: 'crates/crypto-vault/src/sign/ed25519.rs', lines: 700 },
      ],
    },
    {
      name: 'env-cli',
      root: 'crates/env-cli',
      deps: ['core-compiler'],
      files: [
        { path: 'crates/env-cli/src/lib.rs', lines: 240 },
        { path: 'crates/env-cli/src/parse.rs', lines: 160 },
      ],
    },
  ],
  edges: [
    {
      from_district_id: 'agent-orchestrator',
      to_district_id: 'core-compiler',
      from_layer: 2,
      to_layer: 1,
      imported_symbols: ['parse_module', 'TypeCheckResult', 'lower_hir'],
      call_site_count: 14,
      layer_violation: false,
    },
    {
      from_district_id: 'studio-gui',
      to_district_id: 'agent-orchestrator',
      from_layer: 3,
      to_layer: 2,
      imported_symbols: ['ModelRegistryEntry', 'select_model_for_task'],
      call_site_count: 9,
      layer_violation: false,
    },
    {
      from_district_id: 'core-compiler',
      to_district_id: 'storage-db',
      from_layer: 1,
      to_layer: 0,
      imported_symbols: ['ArtifactCasStore', 'SessionContextRow'],
      call_site_count: 6,
      layer_violation: false,
    },
  ],
};

export const URBS_REPO_PRESETS: Record<UrbsRepoPresetId, UrbsRepoPreset> = {
  live: {
    id: 'live',
    label: 'Current Workspace (Live)',
    language: 'Rust / TypeScript',
    scan: DEFAULT_LIVE_FALLBACK_SCAN,
    diagnostics: {
      'crates/core-compiler/src/parser.rs': { warnings: 1, errors: 2 },
      'crates/agent-orchestrator/src/models/registry.rs': { warnings: 2, errors: 0 },
    },
    activeTasks: {
      'AEDILIS-01': {
        taskId: 'T-101',
        agentId: 'AEDILIS-01',
        agentRole: 'Compiler Architect',
        filePath: 'crates/core-compiler/src/parser.rs',
        status: 'Refactoring Pratt expression precedence',
      },
    },
    needsYouGates: [
      {
        id: 'GATE-01',
        district: 'agent-orchestrator',
        title: 'Approve MENS model promotion threshold',
      },
    ],
  },
  ts_monorepo: {
    id: 'ts_monorepo',
    label: 'TypeScript Full-Stack Monorepo',
    language: 'TypeScript',
    scan: {
      root: '/workspace/acme-commerce',
      scanned_at_ms: 1729360000000,
      truncated: false,
      repo_name: 'acme-commerce',
      primary_language: 'TypeScript',
      crates: [
        {
          name: 'shared-schema',
          root: 'packages/shared-schema',
          deps: [],
          dirty_file_count: 1,
          total_churn_lines: 38,
          files: [
            {
              path: 'packages/shared-schema/src/order.ts',
              lines: 420,
              git_status: 'M',
              diff_added_lines: 30,
              diff_deleted_lines: 8,
            },
            { path: 'packages/shared-schema/src/customer.ts', lines: 310 },
            { path: 'packages/shared-schema/src/money.ts', lines: 240 },
          ],
        },
        {
          name: 'auth-vault',
          root: 'packages/auth-vault',
          deps: ['shared-schema'],
          files: [
            { path: 'packages/auth-vault/src/jwt.ts', lines: 540 },
            { path: 'packages/auth-vault/src/oauth.ts', lines: 620 },
            { path: 'packages/auth-vault/src/session_store.ts', lines: 480 },
          ],
        },
        {
          name: 'checkout-engine',
          root: 'packages/checkout-engine',
          deps: ['shared-schema', 'auth-vault'],
          dirty_file_count: 2,
          total_churn_lines: 84,
          files: [
            {
              path: 'packages/checkout-engine/src/pricing_compiler.ts',
              lines: 1450,
              git_status: 'M',
              diff_added_lines: 52,
              diff_deleted_lines: 14,
            },
            {
              path: 'packages/checkout-engine/src/tax_rules.ts',
              lines: 980,
              git_status: 'M',
              diff_added_lines: 14,
              diff_deleted_lines: 4,
            },
            { path: 'packages/checkout-engine/src/discounts.ts', lines: 760 },
            { path: 'packages/checkout-engine/src/cart_state.ts', lines: 810 },
          ],
        },
        {
          name: 'api-router',
          root: 'services/api-router',
          deps: ['checkout-engine', 'auth-vault'],
          dirty_file_count: 1,
          total_churn_lines: 29,
          files: [
            {
              path: 'services/api-router/src/trpc_hub.ts',
              lines: 1120,
              git_status: 'M',
              diff_added_lines: 22,
              diff_deleted_lines: 7,
            },
            { path: 'services/api-router/src/webhooks.ts', lines: 690 },
            { path: 'services/api-router/src/rate_limit.ts', lines: 430 },
          ],
        },
        {
          name: 'web-storefront',
          root: 'apps/web-storefront',
          deps: ['api-router', 'shared-schema'],
          files: [
            { path: 'apps/web-storefront/src/CheckoutPage.tsx', lines: 1340 },
            { path: 'apps/web-storefront/src/CartDrawer.tsx', lines: 890 },
            { path: 'apps/web-storefront/src/ProductGrid.tsx', lines: 920 },
          ],
        },
        {
          name: 'e2e-playwright-bench',
          root: 'packages/e2e-playwright-bench',
          deps: ['web-storefront'],
          files: [
            { path: 'packages/e2e-playwright-bench/checkout.spec.ts', lines: 520 },
            { path: 'packages/e2e-playwright-bench/auth.spec.ts', lines: 410 },
          ],
        },
      ],
    },
    diagnostics: {
      'packages/checkout-engine/src/pricing_compiler.ts': { warnings: 1, errors: 1 },
    },
    activeTasks: {
      'TS-AGENT-01': {
        taskId: 'TS-201',
        agentId: 'TS-AGENT-01',
        agentRole: 'Full-Stack Builder',
        filePath: 'packages/checkout-engine/src/pricing_compiler.ts',
        status: 'Updating multi-currency Stripe intent schema',
      },
    },
    needsYouGates: [
      {
        id: 'GATE-TS-1',
        district: 'api-router',
        title: 'Approve webhook signature rotation migration',
      },
    ],
  },
  python_ml: {
    id: 'python_ml',
    label: 'Python FastAPI + ML Service',
    language: 'Python',
    scan: {
      root: '/workspace/neural-rag-platform',
      scanned_at_ms: 1729360000000,
      truncated: false,
      repo_name: 'neural-rag-platform',
      primary_language: 'Python',
      crates: [
        {
          name: 'core_schema',
          root: 'src/core_schema',
          deps: [],
          files: [
            { path: 'src/core_schema/models.py', lines: 480 },
            { path: 'src/core_schema/config.py', lines: 260 },
          ],
        },
        {
          name: 'pgvector_store',
          root: 'src/pgvector_store',
          deps: ['core_schema'],
          dirty_file_count: 1,
          total_churn_lines: 47,
          files: [
            {
              path: 'src/pgvector_store/index_vault.py',
              lines: 920,
              git_status: 'M',
              diff_added_lines: 36,
              diff_deleted_lines: 11,
            },
            { path: 'src/pgvector_store/migrations.py', lines: 540 },
          ],
        },
        {
          name: 'rag_embeddings_scholar',
          root: 'src/rag_embeddings_scholar',
          deps: ['pgvector_store', 'core_schema'],
          dirty_file_count: 1,
          total_churn_lines: 64,
          files: [
            {
              path: 'src/rag_embeddings_scholar/reranker_model.py',
              lines: 1380,
              git_status: 'M',
              diff_added_lines: 49,
              diff_deleted_lines: 15,
            },
            { path: 'src/rag_embeddings_scholar/chunker.py', lines: 780 },
            { path: 'src/rag_embeddings_scholar/lora_trainer.py', lines: 1120 },
          ],
        },
        {
          name: 'fastapi_orchestrator',
          root: 'src/fastapi_orchestrator',
          deps: ['rag_embeddings_scholar', 'pgvector_store'],
          files: [
            { path: 'src/fastapi_orchestrator/router.py', lines: 1050 },
            { path: 'src/fastapi_orchestrator/worker_queue.py', lines: 840 },
          ],
        },
        {
          name: 'gradio_dashboard_ui',
          root: 'src/gradio_dashboard_ui',
          deps: ['fastapi_orchestrator'],
          files: [
            { path: 'src/gradio_dashboard_ui/portal_view.py', lines: 760 },
            { path: 'src/gradio_dashboard_ui/metrics_panel.py', lines: 520 },
          ],
        },
        {
          name: 'rag_eval_bench',
          root: 'src/rag_eval_bench',
          deps: ['rag_embeddings_scholar'],
          files: [
            { path: 'src/rag_eval_bench/test_recall.py', lines: 610 },
            { path: 'src/rag_eval_bench/bench_latency.py', lines: 440 },
          ],
        },
      ],
    },
    diagnostics: {
      'src/rag_embeddings_scholar/reranker_model.py': { warnings: 1, errors: 1 },
    },
    activeTasks: {
      'PY-SCHOLAR-01': {
        taskId: 'PY-301',
        agentId: 'PY-SCHOLAR-01',
        agentRole: 'ML Researcher',
        filePath: 'src/rag_embeddings_scholar/reranker_model.py',
        status: 'Tuning BGE cross-encoder batch tensor stride',
      },
    },
    needsYouGates: [
      {
        id: 'GATE-PY-1',
        district: 'pgvector_store',
        title: 'Confirm HNSW ef_construction index rebuild',
      },
    ],
  },
  go_microservices: {
    id: 'go_microservices',
    label: 'Go Distributed Microservices',
    language: 'Go',
    scan: {
      root: '/workspace/atlas-ledger-mesh',
      scanned_at_ms: 1729360000000,
      truncated: false,
      repo_name: 'atlas-ledger-mesh',
      primary_language: 'Go',
      crates: [
        {
          name: 'pkg-proto-core',
          root: 'pkg/pkg-proto-core',
          deps: [],
          files: [
            { path: 'pkg/pkg-proto-core/ledger.pb.go', lines: 950 },
            { path: 'pkg/pkg-proto-core/envelope.go', lines: 420 },
          ],
        },
        {
          name: 'internal-vault-db',
          root: 'internal/internal-vault-db',
          deps: ['pkg-proto-core'],
          files: [
            { path: 'internal/internal-vault-db/cockroach_store.go', lines: 1180 },
            { path: 'internal/internal-vault-db/merkle_crypto.go', lines: 890 },
          ],
        },
        {
          name: 'internal-matching-engine',
          root: 'internal/internal-matching-engine',
          deps: ['internal-vault-db', 'pkg-proto-core'],
          dirty_file_count: 1,
          total_churn_lines: 72,
          files: [
            {
              path: 'internal/internal-matching-engine/orderbook_kernel.go',
              lines: 1680,
              git_status: 'M',
              diff_added_lines: 55,
              diff_deleted_lines: 17,
            },
            { path: 'internal/internal-matching-engine/risk_check.go', lines: 940 },
          ],
        },
        {
          name: 'internal-settlement-orchestrator',
          root: 'internal/internal-settlement-orchestrator',
          deps: ['internal-matching-engine', 'internal-vault-db'],
          files: [
            { path: 'internal/internal-settlement-orchestrator/raft_coordinator.go', lines: 1320 },
            { path: 'internal/internal-settlement-orchestrator/dispatcher.go', lines: 870 },
          ],
        },
        {
          name: 'cmd-edge-gateway',
          root: 'cmd/cmd-edge-gateway',
          deps: ['internal-settlement-orchestrator'],
          files: [
            { path: 'cmd/cmd-edge-gateway/main.go', lines: 540 },
            { path: 'cmd/cmd-edge-gateway/grpc_ingress.go', lines: 680 },
          ],
        },
        {
          name: 'bench-load-arena',
          root: 'internal/bench-load-arena',
          deps: ['internal-matching-engine'],
          files: [
            { path: 'internal/bench-load-arena/orderbook_test.go', lines: 710 },
          ],
        },
      ],
    },
    diagnostics: {
      'internal/internal-matching-engine/orderbook_kernel.go': { warnings: 0, errors: 1 },
    },
    activeTasks: {
      'GO-LEGIO-01': {
        taskId: 'GO-401',
        agentId: 'GO-LEGIO-01',
        agentRole: 'Systems Engineer',
        filePath: 'internal/internal-matching-engine/orderbook_kernel.go',
        status: 'Eliminating lock contention in price-time priority ring',
      },
    },
    needsYouGates: [
      {
        id: 'GATE-GO-1',
        district: 'internal-settlement-orchestrator',
        title: 'Approve Raft quorum snapshot interval change',
      },
    ],
  },
};
