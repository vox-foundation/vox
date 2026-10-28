/**
 * Reusable Tauri-invoke mock for vox-gui e2e screenshot sweeps.
 *
 * `installTauriMock(viewKey)` is injected via `addMockInitScript` (e2e/lib/tauriMockShared.ts),
 * not raw `page.addInitScript`. It forces the target surface (localStorage `vox_workbench_tabs.v1`
 * + the `get_initial_view` command) and installs a rich `window.__TAURI_INTERNALS__.invoke` mock
 * so every panel renders with representative data in a bare browser (no Tauri host). The body is
 * the verbatim mock previously inlined in `screenshots.spec.ts` — keep EVERY command case in sync.
 */
export function installTauriMock(viewKey: string): void {
  const shared = (window as any).__VOX_MOCK_SHARED__;
  if (!shared) {
    throw new Error(
      'installTauriMock must be injected via addMockInitScript (e2e/lib/tauriMockShared.ts)',
    );
  }
  shared.seedMockEnvironment(viewKey);

  const modelIds = ['mens-8b', 'anthropic/claude-opus', 'anthropic/claude-sonnet', 'anthropic/claude-haiku', 'qwen-coder-7b', 'local-llama'];
  const modelNames = ['Mens 8B', 'Claude Opus', 'Claude Sonnet', 'Claude Haiku', 'Qwen Coder 7B', 'Local Llama'];
  const models = Array.from({ length: 6 }, (_, i) => ({
    id: modelIds[i],
    // ModelsView reads `id`; harness route is HarnessRedirect (composer parity). Provide all model keys.
    model_id: modelIds[i],
    display_name: modelNames[i],
    provider: ['mens', 'anthropic', 'anthropic', 'anthropic', 'local', 'ollama'][i],
    tier: ['Local', 'Elite', 'Pro', 'Fast', 'Free', 'Local'][i],
    cost_per_1k: [0, 0.015, 0.003, 0.0008, 0, 0][i],
    max_tokens: 200000,
    is_free: [true, false, false, false, true, true][i],
    latency_p50_ms: [120, 900, 600, 300, 200, 150][i],
    success_rate: [0.98, 0.99, 0.985, 0.97, 0.95, 0.93][i],
    quality_score: [0.78, 0.95, 0.9, 0.82, 0.7, 0.65][i],
  }));
  models.push({
    id: 'mens/e2e-smoke-metal',
    model_id: 'mens/e2e-smoke-metal',
    display_name: 'E2E Smoke Metal',
    provider: 'mens',
    tier: 'Local',
    cost_per_1k: 0,
    max_tokens: 200000,
    is_free: true,
    latency_p50_ms: 110,
    success_rate: 0.97,
    quality_score: 0.76,
  });

  const queueSnapshot = {
    candidates: {
      total: 14,
      by_class: { algorithmic_improvement: 5, reproducibility_infra: 4, telemetry_trust: 3, policy_governance: 2 },
      top_5_by_confidence: Array.from({ length: 5 }, (_, i) => ({
        candidate_id: `cand-${100 + i}`, candidate_class: 'algorithmic_improvement',
        confidence: 0.9 - i * 0.07, state: 'evidence_incomplete',
        created_at_ms: 1717000000000, updated_at_ms: 1717400000000,
      })),
    },
    claims_pending: { verifiable: 23, abstained: 6, extraction_running: 2 },
    manifests_in_reply_window: ['pub-7', 'pub-9'],
    retraction_queue: ['pub-3'],
    stalls: [{ candidate_id: 'cand-77', class: 'telemetry_trust', stuck_for_ms: 3200000000 }],
  };

  const searchResponse = {
    hits: Array.from({ length: 8 }, (_, i) => ({
      source: ['memory', 'chunk', 'repo', 'web', 'knowledge', 'memory', 'chunk', 'repo'][i],
      kind: ['memory', 'doc', 'code', 'web', 'knowledge', 'memory', 'doc', 'code'][i],
      path: ['MEMORY.md', 'docs/architecture/search.md', 'crates/vox-search/src/execution.rs',
             'https://example.com/hybrid-search', 'node:retrieval', 'feedback_no_stubs.md',
             'docs/spec.md', 'crates/vox-gui/src/commands/search.rs'][i],
      title: ['Memory index', 'Search design', 'execute_search_plan', 'Hybrid Search', 'retrieval node', null, null, null][i],
      snippet: 'the hybrid search engine fuses bm25 and vector recall with rrf over the candidate set',
      score: 0.95 - i * 0.08,
      provenance: ['bm25', 'vector'],
      locator: { kind: ['memory', 'file', 'file', 'web', 'memory', 'memory', 'file', 'file'][i], value: 'x' },
    })),
    facets_by_source: [{ value: 'memory', count: 3 }, { value: 'chunk', count: 2 }, { value: 'repo', count: 2 }, { value: 'web', count: 1 }],
    facets_by_kind: [{ value: 'doc', count: 2 }, { value: 'code', count: 2 }, { value: 'memory', count: 3 }, { value: 'web', count: 1 }],
    total: 23, next_cursor: 8, corpora: ['memory', 'documentchunks', 'repoinventory', 'webresearch'],
  };

  const ludusProfile = {
    user_id: 'local', level: 27, xp: 4200, xp_to_next_level: 800, xp_progress: 0.62,
    total_xp_earned: 91000, crystals: 1840, lumens: 320, energy: 80, max_energy: 120,
    current_streak: 9, prestige_level: 1, title: 'Centurio', full_title: 'Ascendant Centurio', trust_tier: 'Proven',
  };

  const manifests = Array.from({ length: 10 }, (_, i) => ({
    publication_id: `pub-${i + 1}`, content_type: 'paper',
    state: ['draft', 'draft', 'doi_reserved', 'approved', 'approved', 'submitted', 'submitted', 'published', 'published', 'failed'][i],
    created_at_ms: 1717000000000, updated_at_ms: 1717400000000,
  }));

  const sessions = Array.from({ length: 6 }, (_, i) => ({
    id: i + 1, status: ['completed', 'completed', 'failed', 'active', 'completed', 'orphaned'][i],
    query_text: ['vector db tradeoffs', 'rrf fusion weights', 'tantivy vs qdrant', 'embedding drift', 'crag routing', 'eval harness'][i],
    started_at_ms: 1717000000000, finished_at_ms: 1717400000000,
  }));

  (window as any).__MOCK_APPROVALS__ = [
    {
      approval_id: 'AP-000001',
      tool: 'vox_run_shell',
      summary: 'rm -rf build',
      requested_at_ms: 1717400000000,
      resolved: false,
    },
  ];

  (window as any).__MOCK_HOPPER__ = [] as any[];

  (window as any).__MOCK_SESSIONS__ = [
    { session_id: 'mock-session-1', title: 'Mock chat', updated_at: 'now', message_count: 2, conversation_id: 1 },
  ] as any[];

  (window as any).__MOCK_VCS_ISOLATION__ = {
    strategy_default: 'shared_branch',
    per_agent: { '1': 'shared_branch', '3': 'separate_branches' },
    active_conflicts: [
      { id: 'C-101', path: 'acme/payments-service/src/webhooks/retry.rs', sides: ['1', '3'], created_ms: 1717400000000 },
    ],
  };

  const mcpResult = (tool: string, targs?: any) => {
    if (tool.includes('mesh_nodes')) return { nodes: [{ id: 'node-a', status: 'online', vram_gb: 24 }, { id: 'node-b', status: 'online', vram_gb: 12 }], edges: [] };
    if (tool.includes('resolve_approval')) {
      const id = String(targs?.approval_id ?? '');
      const hit = ((window as any).__MOCK_APPROVALS__ as any[]).find(a => a.approval_id === id);
      if (hit) hit.resolved = true;
      return { success: true, data: { resolved: !!hit } };
    }
    if (tool.includes('pending_approval')) {
      const pending = ((window as any).__MOCK_APPROVALS__ as any[])
        .filter(a => !a.resolved)
        .map(({ resolved: _r, ...a }) => a);
      return { success: true, data: { approvals: pending } };
    }
    if (tool.includes('feedback_list')) {
      return {
        success: true,
        data: {
          needs_you: [
            {
              id: 'FB-204',
              feedback_id: 'FB-204',
              kind: 'clarification',
              prompt: 'Confirm target crate boundary for transport-provider lockfile verification?',
              options: ['vox-arch-check (Recommended)', 'vox-code-audit'],
              gates: [101],
              doubted_task_id: null,
              surface: 'needs_you',
              info_gain_bits: 0.84,
            },
            {
              id: 'FB-205',
              feedback_id: 'FB-205',
              kind: 'doubt',
              prompt: 'Agent-3 flagged potential non-idempotent migration in schema_v4 — overrule or verify?',
              options: [],
              gates: [102],
              doubted_task_id: 102,
              surface: 'needs_you',
              info_gain_bits: 0.91,
            },
          ],
          withheld: [
            {
              id: 'FB-201',
              feedback_id: 'FB-201',
              kind: 'clarification',
              prompt: 'Prefer compact or verbose log timestamps in local test output?',
              options: ['Compact', 'ISO-8601'],
              gates: [],
              doubted_task_id: null,
              surface: 'withheld',
              info_gain_bits: 0.08,
            },
          ],
        },
      };
    }
    if (tool.includes('git_diff')) return { success: true, data: 'diff --git a/README.md b/README.md\n' };
    if (tool === 'vox_skill_list') {
      return {
        success: true,
        data: [
          {
            id: 'superpowers:test-driven-development',
            name: 'test-driven-development',
            version: '2.4.0',
            category: 'engineering',
            description: 'Enforces red-green-refactor verification loops before implementation code.',
            tools: ['vox_test', 'vox_check'],
            source: '.agents/skills/test-driven-development',
            permissions: ['fs:read', 'exec:test'],
            tags: ['tdd', 'verification', 'superpowers'],
          },
          {
            id: 'superpowers:systematic-debugging',
            name: 'systematic-debugging',
            version: '2.4.0',
            category: 'diagnostics',
            description: 'Root-cause hypothesis testing and binary-search fault isolation across crates.',
            tools: ['vox_audit', 'vox_search_query'],
            source: '.agents/skills/systematic-debugging',
            permissions: ['fs:read'],
            tags: ['debugging', 'root-cause', 'superpowers'],
          },
          {
            id: 'vox:cv-flywheel-audit',
            name: 'cv-flywheel-audit',
            version: '1.9.0',
            category: 'gui-quality',
            description: 'Sobel/RMS pixel contrast + axe-core WCAG 2.2 AA & layout bounds convergence.',
            tools: ['browser_snapshot', 'browser_validate_playwright'],
            source: '.vox/skills/cv-flywheel-audit',
            permissions: ['browser:control'],
            tags: ['cv', 'playwright', 'wcag'],
          },
        ],
      };
    }
    if (tool === 'vox_plugin_list') {
      return {
        success: true,
        data: [
          {
            id: 'vox-plugin-superpowers',
            version: '2.4.0',
            payload_kind: 'skill-bundle',
            install_dir: 'crates/vox-skills/skills/superpowers',
          },
          {
            id: 'vox-plugin-browser-songhand',
            version: '1.2.0',
            payload_kind: 'mcp-bridge',
            install_dir: 'crates/vox-browser',
          },
        ],
      };
    }
    if (tool.includes('skill') || tool.includes('plugin')) return { skills: [{ id: 'superpowers', name: 'Superpowers', enabled: true }], plugins: [{ id: 'design', name: 'Design' }] };
    if (tool.includes('vox_search_status')) {
      return {
        success: true,
        data: {
          default_corpus_id: 'workspace-code',
          ttl_days: 7,
          ttl_days_contract: 7,
          ttl_days_env_forced: false,
          corpora: [
            {
              corpus_id: 'workspace-code',
              title: 'Workspace Code & AST Graph',
              is_fresh: true,
              node_count: 14820,
              edge_count: 42310,
              built_at: new Date(Date.now() - 3600_000).toISOString(),
              stale_reasons: [],
            },
            {
              corpus_id: 'architecture-ssot',
              title: 'Architecture & KB Docs',
              is_fresh: true,
              node_count: 3420,
              edge_count: 8910,
              built_at: new Date(Date.now() - 7200_000).toISOString(),
              stale_reasons: [],
            },
          ],
        },
      };
    }
    return { ok: true };
  };

  (window as any).__TAURI_INTERNALS__ = {
    ...((window as any).__TAURI_INTERNALS__ || {}),
    invoke: async (cmd: string, args?: any) => {
      (window as any).__VOX_IPC_ACTIVE_COUNT__ = ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 0) + 1;
      (window as any).__TAURI_CALLS__.push({ cmd, args: args ?? null });
      try {
        switch (cmd) {
        case 'get_models_snapshot': {
          // Composed from the per-command fixtures so tauriMockRich's overrides flow through.
          const inv = (window as any).__TAURI_INTERNALS__.invoke;
          const [cards, routing, active_model, providers] = await Promise.all([
            inv('list_model_cards'), inv('get_routing_summary_live'), inv('get_active_model'), inv('inference_provider_status'),
          ]);
          return { cards, routing, active_model, providers };
        }
        case 'list_model_cards': return models;
        case 'get_active_model': return 'anthropic/claude-sonnet';
        case 'get_auto_model_recommendation':
          return {
            selected_model_id: 'mens/runs/qwen3_27b_metal_check/quant_q6_k',
            detected_vram_gb: 27.0,
            tier_reason: 'High-fidelity 6-bit quantized 27B',
          };
        case 'get_routing_summary_live':
          // Family keys from the bootstrap catalog (see e2e/lib/tauriMock.families.test.ts); no live catalog in e2e.
          return {
            active_model: 'anthropic/claude-sonnet', exploration_spent_usd: 2.4, exploration_budget_usd: 50,
            arm_count: 6, model_count: 7,
            family: 'anthropic/claude-sonnet', resolved_from: 'bootstrap', reason: 'lowest cost that fits the mode',
            decision_preview: { selected_model: 'anthropic/claude-sonnet', discovery_state: 'confirmed',
              alternatives: ['anthropic/claude-haiku', 'deepseek/deepseek-flash'], rejection_reasons: ['budget cap'],
              intelligence_score: 0.92, efficiency_score: 0.7, latency_score: 0.6 },
          };
        case 'explain_routing':
          return {
            mode: args?.mode ?? 'efficiency',
            task: args?.task ?? 'codegen',
            complexity: args?.complexity ?? 7,
            chosen: 'acme/widget-5.5',
            only_candidate: false,
            total_models: 120,
            candidates: [
              {
                id: 'acme/widget-5.5',
                provider: 'acme',
                tier: 'Fast',
                is_free: false,
                price_out_per_m: 0.9,
                score: 0.65,
                quality: { value: 0.6, source: 'inherited', index: 38.2, inherited_from: 'acme/widget-5.0' },
                parts: { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 },
              },
              {
                id: 'acme/widget-9',
                provider: 'acme',
                tier: 'Pro',
                is_free: false,
                price_out_per_m: null,
                score: 0.61,
                quality: { value: 0.8, source: 'benchmark', index: 46.3, inherited_from: null },
                parts: { quality: 0.3, efficiency: 0.2, latency: 0.1, other: 0.05, bonuses: 0 },
              },
            ],
            excluded: [
              { reason: 'flagship_excluded_by_mode', count: 4, examples: ['acme/widget-flagship-9', 'acme/widget-flagship-8'] },
              { reason: 'superseded', count: 1, examples: ['acme/widget-4.8'] },
            ],
          };
        case 'get_routing_health':
          return {
            schema_version: 1,
            checked_at_unix: 0,
            models: 120,
            cloud_models: 100,
            benchmarked: 40,
            inherited: 7,
            unknown_tier_cloud: 0,
            quality_scale: 'derived',
            price_bands: 'derived',
            efficient_pick: 'acme/widget-5.5',
            violations: [],
          };
        case 'get_selection_policy': return { chain: ['anthropic/claude-opus', 'anthropic/claude-sonnet', 'anthropic/claude-haiku'], free_tier: true };
        case 'get_routing_intentions': return [
          { id: 'axis-quality', parent: 'Quality', branch: 'Opus', phase: 'Validated', conf: 0.92, note: 'Highest reasoning tier' },
          { id: 'axis-speed', parent: 'Latency', branch: 'Haiku', phase: 'Active', conf: 0.74, note: 'Fast path for chat' },
          { id: 'axis-cost', parent: 'Budget', branch: 'Local', phase: 'Speculative', conf: 0.58, note: 'Free local tier' },
        ];
        case 'policy_list': return [
          { id: 'pol-1', domain: 'security', group: 'crypto', title: 'No weak AEAD', severity: 'error', blocking: true, protected: false },
          { id: 'pol-2', domain: 'ci', group: 'runner', title: 'Self-hosted default', severity: 'warn', blocking: false, protected: false },
        ];
        case 'list_branches': return [{ branch: 'main', path: '.', isCurrent: true }];
        case 'mercatus_load_config': return {
          _meta: {
            sources: {
              openrouter: { enabled: true, costUsd: 0.002, cadenceHours: 6, tier: 'live' },
              vast_ai: { enabled: true, costUsd: 0.005, cadenceHours: 12, tier: 'spot' },
              runpod: { enabled: true, costUsd: 0.004, cadenceHours: 12, tier: 'spot' },
            },
          },
          watchlist: [
            {
              id: 'rtx-4090-24gb',
              role: 'MENS QLoRA trainer',
              model: 'NVIDIA RTX 4090 24GB',
              sources: ['vast_ai', 'runpod'],
              ids: { vast_ai: 'gpu-4090', runpod: 'rtx4090' },
              target_usd: 0.39,
            },
            {
              id: 'claude-sonnet-4',
              role: 'Frontier verifier',
              model: 'anthropic/claude-sonnet',
              sources: ['openrouter'],
              ids: { openrouter: 'anthropic/claude-sonnet' },
              target_usd: 3.0,
            },
          ],
        };
        case 'list_subagent_tree': return [
          { task_id: 101, agent_id: 1, parent_agent_id: null, reason: 'Coordinate CV Flywheel & Chat Dock audit' },
          { task_id: 102, agent_id: 2, parent_agent_id: 1, reason: 'Verify WCAG 2.2 AA contrast & DOM bounds' },
        ];
        case 'policy_status': return [
          { id: 'pol-1', branch: 'main', status: 'pass', hits: 0 },
          { id: 'pol-2', branch: 'main', status: 'warn', hits: 2 },
        ];
        case 'policy_show': return {
          id: args?.id ?? 'pol-1',
          domain: 'security',
          group: 'crypto',
          title: 'No weak AEAD',
          description: 'Use vox-crypto AEAD only.',
          severity: 'error',
          blocking: true,
          protected: false,
          runsOn: ['main'],
          origin: 'vox-rule-pack',
          docs: 'docs/src/reference/secrets-ssot.md',
          sourceKind: 'rule-pack',
          sourceRef: 'crypto/no-weak-aead',
          sourceDetail: null,
        };
        case 'get_model_scoreboard': return models.map((m, i) => ({
          model_id: m.id,
          task_category: ['code', 'research', 'chat', 'plan', 'code', 'chat'][i],
          strength_tag: ['speed', 'quality', 'balanced', 'quality', 'speed', 'balanced'][i],
          n_calls: [120, 80, 60, 40, 30, 20][i],
          success_rate: m.success_rate,
          p50_latency_ms: m.latency_p50_ms,
          cost_per_success_usd: [0.0, 0.02, 0.004, 0.001, 0.0, 0.0][i],
          quality_score: m.quality_score,
        }));
        case 'explain_model_selection': return { chosen: 'anthropic/claude-sonnet', reason: 'lowest cost that fits the mode' };
        case 'suggest_model_for_task': return 'anthropic/claude-sonnet';
        case 'get_ludus_profile': return ludusProfile;
        case 'list_ludus_notifications': return [
          { id: 'n1', level: 'ok', title: 'Level up! → 27', message: 'Reached Centurio', created_at: 1717400000000, kind: 'LevelUp' },
          { id: 'n2', level: 'ok', title: 'Achievement: Bug Slayer', message: 'Fixed 10 bugs', created_at: 1717400000000, kind: 'AchievementUnlocked' },
          { id: 'n3', level: 'warn', title: 'Streak at risk', message: 'Code today to keep your 9-day streak', created_at: 1717400000000, kind: 'StreakLost' },
        ];
        case 'get_gamify_settings': return { enabled: true, mode: 'balanced' };
        case 'list_gamify_leaderboard': return Array.from({ length: 6 }, (_, i) => ({
          rank: i + 1, user_id: ['archon', 'nova', 'cipher', 'quill', 'atlas', 'echo'][i],
          level: [27, 25, 22, 19, 17, 14][i], score: [91000, 84000, 72000, 60000, 51000, 42000][i],
        }));
        case 'list_gamify_companions': return Array.from({ length: 3 }, (_, i) => ({
          id: `comp-${i + 1}`, name: ['Byte', 'Quill', 'Sprocket'][i], description: null,
          language: ['rust', 'typescript', 'python'][i], mood: ['happy', 'focused', 'sleepy'][i],
          health: [80, 65, 40][i], max_health: 100, energy: [70, 50, 30][i], max_energy: 100,
          code_quality: [0.9, 0.8, 0.7][i], last_active: 1717400000000,
          svg: '<svg viewBox="0 0 32 32"><circle cx="16" cy="16" r="14" fill="#d4af37"/></svg>',
        }));
        case 'list_gamify_quests': return Array.from({ length: 3 }, (_, i) => ({
          id: `quest-${i + 1}`, quest_type: ['daily', 'weekly', 'epic'][i],
          description: ['Fix 3 failing tests', 'Land a refactor PR', 'Ship a new surface'][i],
          hint: ['run vox test', 'keep diffs small', 'register it in the surface registry'][i],
          target: [3, 1, 1][i], progress: [2, 0, 1][i], xp_reward: [150, 400, 1000][i],
          crystal_reward: [10, 40, 120][i], completed: [false, false, true][i],
          status: ['active', 'active', 'completed'][i], expires_at: 1717999999999,
        }));
        case 'vox_search_query': return searchResponse;
        case 'probe_search_provider':
          return {
            provider: args?.provider ?? 'searxng',
            http_status: 200,
            latency_ms: 85,
            success: true,
            hit_count: 3,
            sample_titles: ['Title 1', 'Title 2', 'Title 3'],
            error_message: null,
            remediation_tip: null,
          };
        case 'probe_all_search_providers':
          return ['searxng', 'tavily', 'openalex', 'arxiv', 'wikipedia'].map(p => ({
            provider: p,
            http_status: 200,
            latency_ms: 85,
            success: true,
            hit_count: 3,
            sample_titles: [`${p} hit 1`, `${p} hit 2`],
            error_message: null,
            remediation_tip: null,
          }));
        case 'get_research_engine_status':
          return {
            active_lane: 'fast',
            fast_timeout_ms: 2500,
            deep_timeout_ms: 15000,
            providers: [
              { id: 'wikipedia', name: 'Wikipedia', is_keyless: true, is_enabled: true, has_key: false },
              { id: 'openalex', name: 'OpenAlex', is_keyless: true, is_enabled: true, has_key: false },
              { id: 'arxiv', name: 'arXiv', is_keyless: true, is_enabled: true, has_key: false },
              { id: 'searxng', name: 'SearXNG', is_keyless: true, is_enabled: true, has_key: false },
              { id: 'tavily', name: 'Tavily', is_keyless: false, is_enabled: true, has_key: true, quota_usage: { units_spent: 160, units_limit: 1000, last_synced_at: '2026-09-18' } },
            ],
            free_key_offers: [
              {
                provider_id: 'tavily',
                name: 'Tavily Search',
                signup_url: 'https://app.tavily.com/sign-up',
                free_tier_description: '1,000 queries/month free web search for AI agents and LLMs.',
                quota_summary: '1,000 searches/mo',
                requires_credit_card: false,
                secret_id: 'tavily_api_key',
              },
            ],
          };
        case 'save_research_engine_config':
          return null;
        case 'open_locator': return { action: 'opened' };
        case 'list_research_sessions': return sessions;
        case 'get_research_session_detail': return {
          session: sessions[0],
          report_markdown: '# Findings: Hybrid Retrieval & Vector DB Tradeoffs\n\nVector DBs trade exact recall for sub-millisecond retrieval latency at scale.\n\n- Qdrant: fast HNSW indexing with scalar quantization\n- Tantivy: inverted BM25 index with block-WAND dynamic pruning',
          artifact_json: '{}',
          confidence_tier: 'DeepResearch',
          source_count: 3,
          citation_precision: 1.0,
          claims: [
            {
              claim_id: 'c1',
              text: 'Vector DBs trade exact recall for sub-millisecond retrieval latency at scale.',
              verdict: 'Supported',
              confidence: 0.95,
              resample_stability: 0.92,
              citation_urls: ['https://example.com/vector-db-ann', 'https://example.com/latency-tradeoffs'],
              corroboration_count: 2,
            },
            {
              claim_id: 'c2',
              text: 'Reciprocal Rank Fusion (RRF) with k=60 outperforms naive linear combination of BM25 and dense scores.',
              verdict: 'Supported',
              confidence: 0.92,
              resample_stability: 0.88,
              citation_urls: ['https://example.com/rrf-fusion-benchmark'],
              corroboration_count: 2,
            },
            {
              claim_id: 'c3',
              text: 'Single-source retrieval without corroboration exhibits higher epistemic variance.',
              verdict: 'Contested',
              confidence: 0.65,
              resample_stability: 0.54,
              citation_urls: ['https://example.com/variance-analysis'],
              corroboration_count: 1,
            },
          ],
        };
        case 'list_publication_manifests': return manifests;
        case 'scientia_dashboard_snapshot': return queueSnapshot;
        case 'scientia_cost_rollup':
          return {
            this_quarter: {
              extraction_usd: 1.42,
              critic_usd: 0.88,
              novelty_retrieval_usd: 0.54,
              scholarly_submission_usd: 0.36,
              total_usd: 3.20,
            },
            per_finding_average_usd: 0.23,
            by_provider: [
              { provider: 'anthropic', usd: 2.10 },
              { provider: 'openrouter', usd: 1.10 },
            ],
          };
        case 'coderabbit_token_present':
          return true;
        case 'coderabbit_report':
          return {
            run_state: { chunks: [{ name: 'crate_vox_db', pr_number: 7, status: 'completed' }] },
            db_status: { findings: 3 },
          };
        case 'coderabbit_plan':
          return {
            baseline_branch: 'cr-baseline',
            total_files: 3,
            chunks: [
              { order: 1, name: 'crate_vox_db', files: ['a.rs', 'b.rs'] },
              { order: 2, name: '06_docs_src', files: ['x.md'] },
            ],
          };
        case 'coderabbit_run_async':
          return { status: 'running' };
        case 'get_memory_status': return {
          corpus_counts: { proj: 1280, docs: 540, chats: 96, rules: 210, web: 60 },
          shards: [
            { id: 'proj', depth: 3, entries: 1280, hot: true, dirty: false, spark: [2, 5, 3, 8, 6, 9, 7] },
            { id: 'docs', depth: 2, entries: 540, hot: false, dirty: true, spark: [1, 2, 1, 3, 2, 4, 3] },
            { id: 'chats', depth: 1, entries: 96, hot: false, dirty: false, spark: [0, 1, 0, 2, 1, 1, 2] },
            { id: 'rules', depth: 2, entries: 210, hot: true, dirty: false, spark: [1, 1, 2, 2, 3, 2, 3] },
          ],
        };
        case 'chat_list_sessions':
          return ((window as any).__MOCK_SESSIONS__ as any[]).map(s => ({ ...s }));
        case 'chat_create_session': return { session_id: 'mock-session-new', title: 'New chat', updated_at: 'now', message_count: 0, conversation_id: 2 };
        case 'chat_get_messages': return [{ id: 1, role: 'user', content: 'hello', created_at: 'now', task_id: null }];
        case 'chat_append_message': return 1;
        case 'chat_rename_session': {
          const hit = ((window as any).__MOCK_SESSIONS__ as any[]).find(
            s => s.session_id === String(args?.sessionId),
          );
          if (hit) hit.title = String(args?.title ?? hit.title);
          return null;
        }
        case 'chat_archive_session': {
          (window as any).__MOCK_SESSIONS__ = ((window as any).__MOCK_SESSIONS__ as any[]).filter(
            s => s.session_id !== String(args?.sessionId),
          );
          return null;
        }
        case 'get_command_catalog': return {
          generated_from: 'mock',
          entries: ['check', 'build', 'test', 'run', 'fmt', 'audit', 'research', 'scientia'].map(n => ({
            path: [n], command: `vox ${n}`, about: `Run vox ${n}`, aliases: [], has_subcommands: false,
            compiled_in: true, source_group: 'core', feature_gate: null, tier: 'recommended',
            arguments: [{ name: 'path', short: null, long: 'path', help: 'Target path', required: false, takes_value: true, value_kind: 'value', possible_values: [], default_values: [] }],
          })),
        };
        case 'get_full_registry': return { commands: [] };
        case 'vox_docs_index':
          return [
            {
              title: 'CLI Reference',
              description: 'Vox command-line interface',
              path: 'docs/src/reference/cli.md',
            },
          ];
        case 'read_doc_markdown':
          return `# ${String(args?.path ?? 'doc')}\n\nMock documentation body for visual audit.`;
        case 'get_command_metadata': return { safety_class: 'read_only', confirmation_policy: 'none' };
        case 'list_gui_runs': return Array.from({ length: 5 }, (_, i) => ({
          run_id: `gui-run-${i + 1}`, workflow_name: ['gui.harness.submit', 'gui.policy.doubt', 'gui.search', 'gui.research', 'gui.repo'][i],
          status: ['success', 'success', 'running', 'failed', 'success'][i], planned_steps: 3, completed_steps: [3, 3, 1, 2, 3][i],
          updated_at_ms: 1717400000000, last_error: i === 3 ? 'exit code 1' : null,
        }));
        case 'get_gui_run': return { run_id: 'gui-run-1', workflow_name: 'gui.harness.submit', status: 'success', steps: [] };
        case 'list_secret_status': return [
          { id: 'ANTHROPIC_API_KEY', present: true, preview: 'sk-...abcd' },
          { id: 'OPENROUTER_API_KEY', present: false, preview: null },
          { id: 'TAVILY_API_KEY', present: true, preview: 'tvly-...wxyz' },
        ];
        case 'get_orchestrator_config': return {};
        case 'secrets_backend_status':
          return { backendMode: 'vault', profile: 'dev', strict: false, available: true, detail: null };
        case 'signing_key_status':
          return { nodeId: 'node-abc', algorithm: 'ed25519', fingerprint: 'fp-deadbeef', pubkeyHex: '00', present: true };
        case 'get_user_config': return null;
        case 'get_llm_config': return {};
        case 'get_llm_spend':
          return { sessionUsd: 0, dayUsd: 0, totalUsd: 0, dailyBudgetUsd: 50, perSessionBudgetUsd: 10 };
        case 'pty_spawn':
        case 'pty_write':
        case 'pty_resize':
        case 'pty_close':
        case 'term_session_input':
        case 'term_session_resize':
        case 'term_session_close':
          return null;
        case 'term_session_open': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const nextIdx = (((window as any).__MOCK_TERM_NEXT_SESSION__ ??= 1) as number) + 1;
          (window as any).__MOCK_TERM_NEXT_SESSION__ = nextIdx;
          const sid = `s-${nextIdx}`;
          store[sid] = {
            id: sid,
            blocks: [],
            replay_utf8: `C:\\workspace\\vox (${sid})> \r\n`,
            seq: 1,
          };
          return sid;
        }
        case 'term_session_list': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          return Object.values(store).map((s: any) => ({
            id: s.id,
            blocks: s.blocks.length,
          }));
        }
        case 'term_session_attach': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const sid = String(args?.sessionId ?? 's-1');
          const sess = store[sid] ?? store['s-1'];
          return {
            session_id: sid,
            blocks: sess.blocks.map((b: any) => ({ ...b })),
            replay_utf8: sess.replay_utf8,
            seq: sess.seq,
          };
        }
        case 'term_session_submit': {
          const store = ((window as any).__MOCK_TERM_SESSIONS__ ??= {
            's-1': {
              id: 's-1',
              blocks: [
                {
                  id: 1,
                  kind: 'Shell',
                  status: 'Ok',
                  origin: { kind: 'user' },
                  input: 'vox run',
                  output: [{ stream: 'Stdout', text: 'Ready.\n', ts_ms: 1 }],
                  exit_code: 0,
                  started_ms: 1,
                  ended_ms: 5,
                },
              ],
              replay_utf8: 'C:\\workspace\\vox> vox run\r\nReady.\r\n',
              seq: 1,
            },
          });
          const sid = String(args?.sessionId ?? 's-1');
          const sess = (store[sid] ??= {
            id: sid,
            blocks: [],
            replay_utf8: '',
            seq: 0,
          });
          const bid = (((window as any).__MOCK_TERM_NEXT_BLOCK__ ??= 1) as number) + 1;
          (window as any).__MOCK_TERM_NEXT_BLOCK__ = bid;
          const rawLine = String(args?.line ?? '').trim();
          const lower = rawLine.toLowerCase();

          const routePrompt = (prompt: string) => {
            const p = prompt.toLowerCase();
            if (p.includes('fix') || p.includes('bug') || p.includes('error') || p.includes('fail')) {
              return { task: 'Debugging', model: 'anthropic/claude-sonnet', tier: 'Pro', axes: 'eff:15/prec:70/lat:15' };
            }
            if (p.includes('plan') || p.includes('architect') || p.includes('design') || p.includes('roadmap')) {
              return { task: 'Planning', model: 'anthropic/claude-opus', tier: 'Elite', axes: 'eff:15/prec:70/lat:15' };
            }
            if (p.includes('why') || p.includes('explain') || p.includes('research') || p.includes('compare')) {
              return { task: 'Research', model: 'anthropic/claude-opus', tier: 'Elite', axes: 'eff:15/prec:70/lat:15' };
            }
            if (p.includes('summarize') || p.includes('classify') || p.includes('parse')) {
              return { task: 'Parsing', model: 'mens-8b', tier: 'Local', axes: 'eff:70/prec:15/lat:15' };
            }
            return { task: 'CodeGen', model: 'anthropic/claude-sonnet', tier: 'Pro', axes: 'eff:33/prec:34/lat:33' };
          };

          let kind = 'Shell';
          let inputText = rawLine;
          let stream = 'Stdout';
          let outText = `Executed: ${rawLine}\n`;
          let exitCode = 0;

          if (rawLine.startsWith('?') || lower.startsWith('/ai ') || lower.startsWith('/agent ')) {
            kind = 'AgentTurn';
            const prompt = rawLine.startsWith('?')
              ? rawLine.slice(1).trim()
              : rawLine.replace(/^\/(ai|agent)\s+/i, '').trim();
            inputText = prompt;
            stream = 'Agent';
            const r = routePrompt(prompt);
            outText = `[routed model=${r.model} task=${r.task} tier=${r.tier} axes=${r.axes}] ${prompt}\n`;
          } else if (rawLine.startsWith('/')) {
            kind = 'SlashCommand';
            if (lower.startsWith('/model route ') || lower.startsWith('/model explain ')) {
              const prompt = rawLine.replace(/^\/model\s+(route|explain)\s+/i, '').trim();
              const r = routePrompt(prompt);
              outText = `[routed model=${r.model} task=${r.task} tier=${r.tier} axes=${r.axes}] ${prompt}\n`;
            } else if (lower === '/help') {
              outText = 'Available commands: /help /model /budget /skills /memory /context /sh /ai\n';
            } else if (lower.startsWith('/model ')) {
              outText = `Model set to: ${rawLine.slice(7).trim()}\n`;
            } else {
              outText = `Slash command executed: ${rawLine}\n`;
            }
          } else if (rawLine.startsWith('!nu') || rawLine.startsWith('/nu')) {
            kind = 'Shell';
            inputText = rawLine;
            outText = '[{"name":"vox-terminal-core","status":"ok"}]\n';
          } else if (rawLine === '40 + 2' || rawLine.startsWith('fn ') || rawLine.startsWith('let ')) {
            kind = 'VoxNative';
            outText = 'Int(42)\n';
          } else {
            kind = 'Shell';
            inputText = rawLine.startsWith('!') ? rawLine.slice(1).trim() : rawLine;
            outText = `[ok] ${inputText}\n`;
          }

          const newBlock = {
            id: bid,
            kind,
            status: exitCode === 0 ? 'Ok' : 'Failed',
            origin: { kind: 'user' },
            input: inputText,
            output: [{ stream, text: outText, ts_ms: Date.now() }],
            exit_code: exitCode,
            started_ms: Date.now() - 5,
            ended_ms: Date.now(),
          };
          sess.blocks.push(newBlock);
          sess.seq += 1;
          sess.replay_utf8 += `C:\\workspace\\vox> ${rawLine}\r\n${outText.replace(/\n/g, '\r\n')}`;
          return bid;
        }
        case 'invoke_mcp_tool': return { tool: args?.tool ?? 'unknown', is_error: false, result: mcpResult(args?.tool ?? '', args?.args) };
        case 'execute_command': {
          const path: string[] = args?.path ?? [];
          const p = path.join(' ');
          if (p === 'scientia dashboard') return { exit_code: 0, stdout: JSON.stringify(queueSnapshot), stderr: '' };
          if (p === 'scientia claims' || p === 'scientia publication-extract-claims')
            return { exit_code: 0, stdout: JSON.stringify({ claims: Array.from({ length: 5 }, (_, i) => ({ claim_id: `c${i}`, text: 'Provider X shows 3% regression under load', verdict: ['Supported', 'Contested', 'Abstain', 'Supported', 'Contradicted'][i], confidence: 0.8 - i * 0.1, verifiability_score: 0.7, numeric: true, verifier_model: 'minicheck' })) }), stderr: '' };
          if (path[0] === 'research') return { exit_code: 0, stdout: 'SearXNG: ok\nDDG: ok\nTavily: ok', stderr: '' };
          if (p === 'mens models') return { exit_code: 0, stdout: 'mens-8b (Q6_K · 6.4 GB) | mens/e2e-smoke-metal (FP16 · 1.2 GB)', stderr: '' };
          if (p === 'mens probe') return { exit_code: 0, stdout: 'GPU backend: Metal/CUDA ready | VRAM: 24.0 GB free | Candle kernel: ok', stderr: '' };
          if (path[0] === 'mens') return { exit_code: 0, stdout: 'training idle | 2 local models | GPU: RTX 4090 (24GB)', stderr: '' };
          if (p === 'populi registry-snapshot') return { exit_code: 0, stdout: 'node-a (online · 24GB VRAM) | node-b (online · 12GB VRAM) | quorum: 2/2', stderr: '' };
          if (path[0] === 'populi') return { exit_code: 0, stdout: 'mesh: 2 nodes online | overlay healthy', stderr: '' };
          if (p === 'oratio status') return { exit_code: 0, stdout: 'session: idle | sample_rate: 16kHz | VAD: silero-v5 ready', stderr: '' };
          if (path[0] === 'oratio') return { exit_code: 0, stdout: 'oratio runtime ok | backend: whisper-local', stderr: '' };
          return { exit_code: 0, stdout: 'ok', stderr: '' };
        }
        case 'submit_orchestrator_task': return { ok: true, task_id: '101', message: 'submitted' };
        // Single composer dispatch (dfb707f38): background turns return a task id to
        // correlate the agent-event stream against; sync turns return the reply itself.
        case 'chat_turn': {
          const background = args?.input?.execution !== 'sync';
          return {
            id: 9001,
            role: 'assistant',
            content: background ? '' : 'Mock quick-chat reply.',
            created_at: new Date().toISOString(),
            task_id: background ? '101' : null,
            model_id: args?.input?.model_override ?? 'anthropic/claude-opus',
            events: [],
          };
        }
        case 'get_task_diff': return 'diff --git a/README.md b/README.md\n';
        case 'list_repo_files': {
          const mockFiles = [
            'README.md',
            'crates/vox-gui/src/main.rs',
            'crates/vox-gui/ui/src/components/surfaces/Loquela/Loquela.tsx',
            'docs/src/reference/cli.md',
          ];
          const q = String(args?.query ?? '').toLowerCase();
          const lim = typeof args?.limit === 'number' ? args.limit : 20;
          const filtered = q
            ? mockFiles.filter(p => p.toLowerCase().includes(q))
            : mockFiles;
          return filtered.slice(0, lim);
        }
        case 'list_orchestrator_tasks': return [];
        case 'workspace_town_scan':
          return {
            root: '/workspace/vox',
            scanned_at_ms: 1729360000000,
            truncated: false,
            crates: [
              {
                name: 'vox-orchestrator',
                root: 'crates/vox-orchestrator',
                files: [
                  { path: 'crates/vox-orchestrator/src/lib.rs', lines: 420 },
                  { path: 'crates/vox-orchestrator/src/models/registry.rs', lines: 310 },
                  { path: 'crates/vox-orchestrator/src/models/select.rs', lines: 280 },
                  { path: 'crates/vox-orchestrator/src/events.rs', lines: 195 },
                ],
              },
              {
                name: 'vox-gui',
                root: 'crates/vox-gui',
                files: [
                  { path: 'crates/vox-gui/src/main.rs', lines: 260 },
                  { path: 'crates/vox-gui/src/commands/chat_turn.rs', lines: 340 },
                  { path: 'crates/vox-gui/src/commands/urbs.rs', lines: 180 },
                ],
              },
              {
                name: 'vox-compiler',
                root: 'crates/vox-compiler',
                files: [
                  { path: 'crates/vox-compiler/src/lib.rs', lines: 510 },
                  { path: 'crates/vox-compiler/src/parser.rs', lines: 640 },
                  { path: 'crates/vox-compiler/src/typeck.rs', lines: 720 },
                ],
              },
              {
                name: 'vox-crypto',
                root: 'crates/vox-crypto',
                files: [
                  { path: 'crates/vox-crypto/src/lib.rs', lines: 210 },
                  { path: 'crates/vox-crypto/src/aead.rs', lines: 145 },
                ],
              },
            ],
          };
        case 'harness_ci_fleet_status':
          return {
            runners: [
              { name: 'legio-mac-01', online: true, busy: true },
              { name: 'legio-linux-02', online: true, busy: false },
              { name: 'legio-win-03', online: true, busy: false },
            ],
            queued: 1,
          };
        case 'vcs_town_status':
          return {
            branches: [
              { name: 'main', is_head: true, track: '[ahead 2]' },
              { name: 'feat/cv-flywheel', is_head: false, track: '[ahead 1]' },
            ],
            prs: [{ number: 108, title: 'Unify Loquela knobs & Urbs Civilization HUD', head_ref: 'feat/cv-flywheel' }],
            prs_available: true,
          };
        case 'mcp_servers_status':
          return [
            { name: 'vox-mcp-core', ok: true },
            { name: 'lean-ctx', ok: true },
            { name: 'gemini-api-docs', ok: true },
          ];
        case 'hopper_list':
          return ((window as any).__MOCK_HOPPER__ as any[]).map(t => ({ ...t }));
        case 'hopper_submit': {
          const items = (window as any).__MOCK_HOPPER__ as any[];
          const n = items.length + 1;
          items.push({
            item_id: `hop-${n}`,
            intent: String(args?.intent ?? ''),
            priority: 1,
            state: 'inbox',
            task_id: 9000 + n,
          });
          return { item_id: `hop-${n}` };
        }
        case 'hopper_cancel': {
          const items = (window as any).__MOCK_HOPPER__ as any[];
          (window as any).__MOCK_HOPPER__ = items.filter(t => t.item_id !== String(args?.itemId));
          return null;
        }
        case 'hopper_reprioritize': {
          const items = (window as any).__MOCK_HOPPER__ as any[];
          const hit = items.find(t => t.item_id === String(args?.itemId));
          if (hit) hit.priority = Number(args?.priority ?? 1);
          return null;
        }
        case 'hopper_mark_done': {
          const items = (window as any).__MOCK_HOPPER__ as any[];
          const hit = items.find(t => t.item_id === String(args?.itemId));
          if (hit) hit.state = 'done';
          return hit ? { ...hit } : null;
        }
        case 'inference_provider_status': return [{ provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null, local_models: [] }, { provider: 'Ollama', key_present: true, is_local: true, local_reachable: true, local_models: ['local-llama-small'] }, { provider: 'Mens', key_present: true, is_local: true, local_reachable: true, local_models: ['mens-8b', 'mens/e2e-smoke-metal'] }];
        case 'set_active_model': return null;
        case 'get_vcs_isolation':
          return { ...((window as any).__MOCK_VCS_ISOLATION__ ?? { strategy_default: 'shared_branch', per_agent: {}, active_conflicts: [] }) };
        case 'set_vcs_isolation_strategy': {
          const cur = (window as any).__MOCK_VCS_ISOLATION__ ?? { strategy_default: 'shared_branch', per_agent: {}, active_conflicts: [] };
          if (args?.default) cur.strategy_default = String(args.default);
          (window as any).__MOCK_VCS_ISOLATION__ = cur;
          return { ...cur };
        }
        case 'get_archive_status': return { swhid: null, swh_task_id: null, swh_task_status: null, zenodo_doi: null, zenodo_state: null };
        case 'get_completion_report': return { score: 100, warnings: [], is_complete: true };
        case 'harness_eval_history':
          return [
            {
              run_id: 'eval-2026-1019-nightly',
              git_sha: '9f4a21c',
              triggered_by: 'ci-nightly',
              pass_count: 24,
              fail_count: 1,
              skip_count: 0,
              total_cost_usd: 0.0842,
              started_at_ms: 1729360000000,
              category_breakdown: [
                { category: 'chat', pass_count: 8, fail_count: 0 },
                { category: 'privacy', pass_count: 5, fail_count: 0 },
                { category: 'tool-calling', pass_count: 6, fail_count: 1 },
                { category: 'compaction', pass_count: 5, fail_count: 0 },
              ],
            },
            {
              run_id: 'eval-2026-1018-smoke',
              git_sha: '7c3e10b',
              triggered_by: 'local-cli',
              pass_count: 25,
              fail_count: 0,
              skip_count: 0,
              total_cost_usd: 0.0615,
              started_at_ms: 1729273600000,
              category_breakdown: [
                { category: 'chat', pass_count: 8, fail_count: 0 },
                { category: 'privacy', pass_count: 5, fail_count: 0 },
                { category: 'tool-calling', pass_count: 7, fail_count: 0 },
                { category: 'compaction', pass_count: 5, fail_count: 0 },
              ],
            },
            {
              run_id: 'eval-2026-1017-mens',
              git_sha: '4d91a0e',
              triggered_by: 'mens-gate',
              pass_count: 22,
              fail_count: 2,
              skip_count: 1,
              total_cost_usd: 0.0,
              started_at_ms: 1729187200000,
              category_breakdown: [
                { category: 'chat', pass_count: 7, fail_count: 1 },
                { category: 'privacy', pass_count: 5, fail_count: 0 },
                { category: 'tool-calling', pass_count: 5, fail_count: 1 },
                { category: 'compaction', pass_count: 5, fail_count: 0 },
              ],
            },
          ];
        case 'harness_eval_regressions':
          return [
            {
              kind: 'CategoryFlip',
              previous_run_id: 'eval-2026-1018-smoke',
              current_run_id: 'eval-2026-1019-nightly',
              previous_git_sha: '7c3e10b',
              current_git_sha: '9f4a21c',
              changed_files: ['crates/vox-orchestrator-mcp/src/dispatch.rs'],
              flipped_task_ids: ['tool-schema-strict-04'],
              detail: 'tool-calling pass rate shifted 7/7 -> 6/7 (+1 schema retry)',
            },
          ];
        case 'activity_query':
          return [
            {
              id: 501,
              ts_ms: 1729363200000,
              kind: 'WorkflowStarted',
              agent_id: 'agent-1 (Aquila)',
              summary: 'Started CV Flywheel 32-surface visual & DOM convergence pass',
              detail_json: '{}',
            },
            {
              id: 502,
              ts_ms: 1729363215000,
              kind: 'TaskCompleted',
              agent_id: 'agent-2 (Bellona)',
              summary: 'Verified WCAG 2.2 AA contrast & 0px overflow across Chat Dock',
              detail_json: '{}',
            },
            {
              id: 503,
              ts_ms: 1729363230000,
              kind: 'CostIncurred',
              agent_id: 'agent-1 (Aquila)',
              summary: 'Cost incurred: $0.0142 (anthropic/claude-sonnet · 4.1k tok)',
              detail_json: '{"CostIncurred":{"cost_usd":0.0142}}',
            },
            {
              id: 504,
              ts_ms: 1729363245000,
              kind: 'BudgetAlert',
              agent_id: 'agent-4 (Drusus)',
              summary: 'Attention budget batching active: 2 low-entropy questions withheld by policy',
              detail_json: '{}',
            },
          ];
        case 'discovery_suggest': {
          const raw = String(args?.typed ?? '').trim();
          const query = raw.replace(/^vox\s*/, '').toLowerCase();
          const catalog = [
            { action_id: 'vox.run', completion: 'run', about: 'Run a Vox script or tool' },
            { action_id: 'vox.check', completion: 'check', about: 'Check a Vox script for type/syntax errors' },
            { action_id: 'vox.build', completion: 'build', about: 'Compile a Vox package to binary' },
            { action_id: 'vox.test', completion: 'test', about: 'Execute unit and doc tests' },
            { action_id: 'vox.config.show', completion: 'config show', about: 'Display resolved configuration values' },
          ];
          return catalog.filter((c) => !query || c.completion.toLowerCase().includes(query));
        }
        case 'discovery_help': {
          const actionId = String(args?.actionId ?? '');
          return {
            action_id: actionId,
            about: `Documentation and options for ${actionId}`,
            args: [{ name: 'target', help: 'Path or package name', required: false }],
            example: actionId.replace(/^vox\./, 'vox '),
          };
        }
        case 'discovery_record': return null;
        case 'browser_session_status':
          return {
            connected: true,
            url: 'http://127.0.0.1:1420',
            title: 'Vox Workbench · Playwright & Songhand Harness',
            viewport: { width: 1280, height: 800 },
            page_id: 'page-vox-1',
            page_count: 1,
          };
        case 'browser_list_pages':
          return [
            {
              page_id: 'page-vox-1',
              url: 'http://127.0.0.1:1420',
              title: 'Vox Workbench · Playwright & Songhand Harness',
            },
          ];
        case 'browser_page_info':
          return {
            page_id: 'page-vox-1',
            url: 'http://127.0.0.1:1420',
            title: 'Vox Workbench · Playwright & Songhand Harness',
            viewport_width: 1280,
            viewport_height: 800,
          };
        case 'browser_snapshot':
          return {
            page_id: 'page-vox-1',
            url: 'http://127.0.0.1:1420',
            title: 'Vox Workbench · Playwright & Songhand Harness',
            tree_text: '[e1] button "Run Songhand"\n[e2] textbox "Instruction"',
            refs: [
              {
                ref_id: 'e1',
                role: 'button',
                name: 'Run Songhand',
                tag: 'button',
                bbox: { x: 120, y: 180, width: 140, height: 32 },
              },
              {
                ref_id: 'e2',
                role: 'textbox',
                name: 'Instruction',
                tag: 'input',
                bbox: { x: 280, y: 180, width: 360, height: 32 },
              },
            ],
          };
        case 'browser_act':
          return {
            ok: true,
            steps_taken: 2,
            final_note: `Executed Songhand instruction: ${String(args?.instruction ?? '')}`,
            transcript: ['snapshot(1) -> [e1] button', 'click_ref(e1) -> ok'],
          };
        case 'browser_extract':
          return {
            ok: true,
            result: `Extracted summary for: ${String(args?.instruction ?? '')}`,
          };
        case 'browser_extract_json':
          return {
            ok: true,
            valid_json: true,
            schema_satisfied: true,
            data: { title: 'Vox Workbench', status: 'healthy', refs: 2 },
          };
        case 'browser_click_ref':
        case 'browser_fill_ref':
        case 'browser_wait_for':
          return { ok: true, ref_id: args?.refId ?? 'e1' };
        case 'browser_read_dom':
          return {
            mode: args?.mode ?? 'text',
            target: args?.target ?? 'body',
            content: 'Vox Workbench Playwright & Songhand Harness Ready',
          };
        case 'browser_validate_playwright':
          return {
            passed: 6,
            failed: 0,
            skipped: 0,
            duration_ms: 1420,
            tests: [
              { title: 'cv-flywheel: 32 surfaces pass WCAG 2.2 AA & layout bounds', file: 'e2e/review/cv-flywheel.spec.ts', status: 'passed', duration_ms: 820, error: null },
              { title: 'browser-harness: Songhand act & extract_json roundtrip', file: 'e2e/review/capture.spec.ts', status: 'passed', duration_ms: 600, error: null },
            ],
            raw_error: null,
          };
        case 'mens_run_reports':
          return {
            eval_local: {
              anti_stub_task_success: 1.0,
              pass_rate_at_k: 1.0,
              placeholder_event_rate: 0.0,
            },
            gate_receipt: {
              overall_passed: true,
              substantive_gate_count: 5,
              failed_gates: [],
              gates: [
                { name: 'pass_at_k', passed: true, message: '1.00 >= 0.85' },
                { name: 'rust_compile_rate', passed: true, message: '1.00 >= 0.95' },
                { name: 'organic_search_recall_at_5', passed: true, message: '1.00 >= 1.00' },
                { name: 'organic_mrr', passed: true, message: '1.00 >= 0.95' },
                { name: 'organic_compilation_pass_rate', passed: true, message: '1.00 >= 1.00' },
              ],
            },
            collateral_damage: {
              status: 'pass',
              failed_on: null,
            },
            spoke_scorecard: {
              schema_version: 'mens_before_after_comparison_v1',
              total_cases: 25,
              before_overall_pass_at_1: 0.24,
              mid_overall_pass_at_1: 0.72,
              overall_pass_at_1: 1.0,
              overall_pass_at_k: 1.0,
              overall_delta_pass_at_1: 0.76,
              monotonic_improvement_verified: true,
              promotion_gate_passed: true,
              promoted_model_id: 'mens/bench25-cycle-2',
              promoted_adapter_uri: 'trusty://mens-adapter/2c9a68b2f0b7',
              spokes: [
                { spoke: 'vox_lang', total_cases: 5, before_pass_at_1: 0.4, mid_pass_at_1: 0.8, pass_at_1: 1.0, pass_at_k: 1.0, delta_pass_at_1: 0.6, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, execution_success_rate: 1.0, curriculum_weight: 0.2 },
                { spoke: 'rust_workspace', total_cases: 5, before_pass_at_1: 0.2, mid_pass_at_1: 0.8, pass_at_1: 1.0, pass_at_k: 1.0, delta_pass_at_1: 0.8, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, execution_success_rate: 1.0, curriculum_weight: 0.2 },
                { spoke: 'structured_shell', total_cases: 5, before_pass_at_1: 0.2, mid_pass_at_1: 0.6, pass_at_1: 1.0, pass_at_k: 1.0, delta_pass_at_1: 0.8, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, execution_success_rate: 1.0, curriculum_weight: 0.2 },
                { spoke: 'mcp_tool_chain', total_cases: 5, before_pass_at_1: 0.2, mid_pass_at_1: 0.8, pass_at_1: 1.0, pass_at_k: 1.0, delta_pass_at_1: 0.8, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, execution_success_rate: 1.0, curriculum_weight: 0.2 },
                { spoke: 'agent_harness', total_cases: 5, before_pass_at_1: 0.2, mid_pass_at_1: 0.6, pass_at_1: 1.0, pass_at_k: 1.0, delta_pass_at_1: 0.8, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, execution_success_rate: 1.0, curriculum_weight: 0.2 },
              ],
            },
            organic_scorecard: {
              schema_version: 'mens_organic_search_spoke_scorecard_v1',
              total_cases: 15,
              overall_search_recall_at_5: 1.0,
              overall_mrr: 1.0,
              overall_grounding_ratio: 1.0,
              overall_compilation_pass_rate: 1.0,
              overall_end_to_end_pass_at_1: 1.0,
              spokes: [
                { spoke: 'vox_lang', total_cases: 3, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'rust_workspace', total_cases: 2, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'structured_shell', total_cases: 2, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'mcp_tool_chain', total_cases: 2, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'agent_harness', total_cases: 2, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'speech_to_code', total_cases: 1, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'populi_mesh', total_cases: 1, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
                { spoke: 'skill_creation', total_cases: 2, search_recall_at_5: 1.0, mean_reciprocal_rank: 1.0, mean_grounding_ratio: 1.0, syntax_validity_rate: 1.0, semantic_validity_rate: 1.0, compilation_pass_rate: 1.0, end_to_end_pass_at_1: 1.0 },
              ],
            },
          };
        default: {
          const ev = shared.eventPluginResponse(cmd, args);
          if (ev !== undefined) return ev;
          return shared.bootstrapResponse(cmd, viewKey);
        }
      }
    } finally {
      (window as any).__VOX_IPC_ACTIVE_COUNT__ = Math.max(
        0,
        ((window as any).__VOX_IPC_ACTIVE_COUNT__ || 1) - 1,
      );
    }
  },
};
}
