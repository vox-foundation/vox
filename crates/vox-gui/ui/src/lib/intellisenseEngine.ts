import { buildSlashEntries, type SlashEntry } from './slashCommands';
import { getLoadedCapabilityIndex, loadCapabilityIndex } from './capabilityIndexLoader';
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';
import { TERM_MENTION_OPTIONS, type TerminalBridgeState } from './terminalBridge';
import type { Suggestion } from '../transport';

export type CompletionCategory =
  | 'all'
  | 'command'
  | 'slash'
  | 'skill'
  | 'term'
  | 'file'
  | 'mcp'
  | 'nu'
  | 'symbol'
  | 'history';

export interface IntelliSenseItem {
  id: string;
  label: string;
  insertText: string;
  ghostSuffix: string;
  category: CompletionCategory;
  badge: string;
  description: string;
  signature?: string;
  actionId?: string;
  score: number;
  meta?: string;
  chip?: {
    kind: string;
    label: string;
    meta?: string;
  };
  slashEntry?: SlashEntry;
}

export interface CompletionTrigger {
  kind:
    | 'vox'
    | 'slash'
    | 'nu'
    | 'pipe'
    | 'prompt'
    | 'at_term'
    | 'at_file'
    | 'at_skill'
    | 'at_mcp'
    | 'at_symbol'
    | 'at_all'
    | 'general';
  query: string;
  replaceStart: number;
  replaceEnd: number;
  rawToken: string;
}

export interface IntelliSenseContext {
  input: string;
  cursorOffset?: number;
  surface: 'terminal' | 'chat';
  shellMode?: 'default' | 'nu' | 'vox';
  catalogSuggestions?: Suggestion[];
  skills?: Array<{
    id?: string;
    name?: string;
    description?: string;
    command?: string;
    capability_id?: string;
    about?: string;
  }>;
  repoFiles?: string[];
  agents?: Array<{ id: string; codename?: string; phase?: string }>;
  termBridge?: TerminalBridgeState;
  history?: string[];
  forceOpen?: boolean;
}

const TERMINAL_COMMAND_REGISTRY: Array<{
  cmd: string;
  usage: string;
  desc: string;
}> = [
  { cmd: '/help', usage: '/help [cmd]', desc: 'List terminal slash commands or show help for one command' },
  { cmd: '/model', usage: '/model [id|route]', desc: 'Inspect or switch the active session LLM model or route' },
  { cmd: '/budget', usage: '/budget', desc: 'Show session token & USD spend vs cap (0% markup)' },
  { cmd: '/skills', usage: '/skills [query]', desc: 'List or filter installed agent skills' },
  { cmd: '/memory', usage: '/memory [query]', desc: 'Search project memory and KB facts' },
  { cmd: '/context', usage: '/context', desc: 'Show active terminal and chat context bundle' },
  { cmd: '/sh', usage: '/sh <command>', desc: 'Run a raw shell command in the active PTY session' },
  { cmd: '/ai', usage: '/ai <prompt>', desc: 'Ask the orchestrator agent using terminal context' },
  { cmd: '/nu', usage: '/nu [pipeline]', desc: 'Show Nushell structured shell idioms or run a pipeline' },
  { cmd: '/security', usage: '/security [l1..l5]', desc: 'View or set terminal security level (L1 ReadOnly .. L5 Autopilot)' },
  { cmd: '/control', usage: '/control [manual|copilot|autopilot]', desc: 'Switch terminal control handoff mode' },
];

const NU_PIPELINE_STAGES: Array<{
  name: string;
  signature: string;
  desc: string;
}> = [
  { name: 'ls', signature: 'ls | where size > 1kb', desc: 'List directory entries as a structured table' },
  { name: 'ps', signature: 'ps | where cpu > 5', desc: 'List host processes with CPU and memory columns' },
  { name: 'open', signature: 'open Cargo.toml', desc: 'Parse TOML/JSON/YAML/CSV file into structured data' },
  { name: 'where', signature: 'where status == "ok"', desc: 'Filter rows of a structured stream by predicate' },
  { name: 'select', signature: 'select name status', desc: 'Project specific columns from a structured table' },
  { name: 'get', signature: 'get package.name', desc: 'Extract a cell or nested path from a record/table' },
  { name: 'to json', signature: 'to json --raw', desc: 'Serialize structured pipeline output to JSON' },
  { name: 'from json', signature: 'from json', desc: 'Parse JSON text into a structured Nushell value' },
  { name: 'sort-by', signature: 'sort-by size --reverse', desc: 'Sort structured table rows by column' },
  { name: 'each', signature: 'each {|it| $it }', desc: 'Map a closure over each element in a stream' },
  { name: 'flatten', signature: 'flatten', desc: 'Flatten nested lists or tables into top-level rows' },
];

const VOX_LSP_SYMBOLS: Array<{
  name: string;
  signature: string;
  desc: string;
}> = [
  { name: '@component', signature: '@component fn View() -> Element', desc: 'Declare a reactive Vox UI component' },
  { name: '@actor', signature: '@actor struct Worker', desc: 'Declare a stateful actor with message handlers' },
  { name: '@workflow', signature: '@workflow fn pipeline() -> Result', desc: 'Declare a durable, replayable workflow' },
  { name: '@activity', signature: '@activity fn step() -> Result', desc: 'Declare a retriable side-effecting activity' },
  { name: '@table', signature: '@table struct Row', desc: 'Declare a VoxDb persistent relational table' },
  { name: '@mcp.tool', signature: '@mcp.tool fn tool_name()', desc: 'Expose a Vox function as a canonical MCP tool' },
  { name: '@gpu', signature: '@gpu fn kernel()', desc: 'Mark a compute function for GPU/WebGPU acceleration' },
  { name: 'fn', signature: 'fn name(arg: str) -> int', desc: 'Declare a pure or effectful Vox function' },
  { name: 'match', signature: 'match expr { pat => body }', desc: 'Exhaustive pattern matching expression' },
];

export function fuzzyScore(query: string, target: string): number {
  const q = query.trim().toLowerCase();
  const t = target.toLowerCase();
  if (!q) return 100;
  if (t === q) return 1000;
  if (t.startsWith(q)) return 850 - Math.min(100, t.length - q.length);
  const idx = t.indexOf(q);
  if (idx >= 0) return 650 - Math.min(150, idx * 5);

  // Subsequence match (nucleo-style character order scoring)
  let qi = 0;
  let consecutive = 0;
  let score = 200;
  for (let ti = 0; ti < t.length && qi < q.length; ti++) {
    if (t[ti] === q[qi]) {
      qi++;
      consecutive++;
      score += 25 + consecutive * 10;
    } else {
      consecutive = 0;
    }
  }
  return qi === q.length ? score : 0;
}

export function parseCompletionTrigger(
  input: string,
  cursorOffset = input.length,
  surface: 'terminal' | 'chat' = 'terminal',
): CompletionTrigger {
  const head = input.slice(0, cursorOffset);
  const trimmedStart = head.trimStart();
  const leadingSpaces = head.length - trimmedStart.length;

  // Check for trailing `@...` token on either surface
  const atMatch = head.match(/(?:^|\s)(@[^\s]*)$/);
  if (atMatch) {
    const rawToken = atMatch[1];
    const replaceStart = head.length - rawToken.length;
    const body = rawToken.slice(1);
    if (body.startsWith('term:') || body.startsWith('term')) {
      return {
        kind: 'at_term',
        query: body.replace(/^term:?/, ''),
        replaceStart,
        replaceEnd: cursorOffset,
        rawToken,
      };
    }
    if (body.startsWith('file:')) {
      return {
        kind: 'at_file',
        query: body.slice('file:'.length),
        replaceStart,
        replaceEnd: cursorOffset,
        rawToken,
      };
    }
    if (body.startsWith('skill:')) {
      return {
        kind: 'at_skill',
        query: body.slice('skill:'.length),
        replaceStart,
        replaceEnd: cursorOffset,
        rawToken,
      };
    }
    if (body.startsWith('mcp:')) {
      return {
        kind: 'at_mcp',
        query: body.slice('mcp:'.length),
        replaceStart,
        replaceEnd: cursorOffset,
        rawToken,
      };
    }
    if (body.startsWith('symbol:')) {
      return {
        kind: 'at_symbol',
        query: body.slice('symbol:'.length),
        replaceStart,
        replaceEnd: cursorOffset,
        rawToken,
      };
    }
    return {
      kind: 'at_all',
      query: body,
      replaceStart,
      replaceEnd: cursorOffset,
      rawToken,
    };
  }

  // Slash command at start of input
  if (trimmedStart.startsWith('/')) {
    return {
      kind: 'slash',
      query: trimmedStart.slice(1).trim(),
      replaceStart: leadingSpaces,
      replaceEnd: cursorOffset,
      rawToken: trimmedStart,
    };
  }

  // Nushell `!nu` or `!`
  if (trimmedStart.startsWith('!nu') || trimmedStart.startsWith('!')) {
    const afterBang = trimmedStart.replace(/^!nu\b\s*/, '').replace(/^!\s*/, '');
    if (afterBang.includes('|')) {
      const pipeIdx = head.lastIndexOf('|');
      const stageRaw = head.slice(pipeIdx + 1);
      const stageTrim = stageRaw.trimStart();
      return {
        kind: 'pipe',
        query: stageTrim,
        replaceStart: head.length - stageTrim.length,
        replaceEnd: cursorOffset,
        rawToken: stageTrim,
      };
    }
    return {
      kind: 'nu',
      query: afterBang.trim(),
      replaceStart: leadingSpaces,
      replaceEnd: cursorOffset,
      rawToken: trimmedStart,
    };
  }

  // Pipe stage in terminal
  if (surface === 'terminal' && head.includes('|')) {
    const pipeIdx = head.lastIndexOf('|');
    const stageRaw = head.slice(pipeIdx + 1);
    const stageTrim = stageRaw.trimStart();
    return {
      kind: 'pipe',
      query: stageTrim,
      replaceStart: head.length - stageTrim.length,
      replaceEnd: cursorOffset,
      rawToken: stageTrim,
    };
  }

  // Agent prompt `?`
  if (trimmedStart.startsWith('?')) {
    return {
      kind: 'prompt',
      query: trimmedStart.slice(1).trim(),
      replaceStart: leadingSpaces,
      replaceEnd: cursorOffset,
      rawToken: trimmedStart,
    };
  }

  // `vox` command
  if (trimmedStart === 'vox' || trimmedStart.startsWith('vox ')) {
    const query = trimmedStart.replace(/^vox\s*/, '');
    return {
      kind: 'vox',
      query,
      replaceStart: leadingSpaces,
      replaceEnd: cursorOffset,
      rawToken: trimmedStart,
    };
  }

  return {
    kind: 'general',
    query: trimmedStart,
    replaceStart: leadingSpaces,
    replaceEnd: cursorOffset,
    rawToken: trimmedStart,
  };
}

function makeGhostSuffix(typed: string, fullReplacement: string): string {
  if (!typed) return '';
  if (fullReplacement.toLowerCase().startsWith(typed.toLowerCase())) {
    return fullReplacement.slice(typed.length);
  }
  return '';
}

export function computeIntelliSenseItems(ctx: IntelliSenseContext): IntelliSenseItem[] {
  const trigger = parseCompletionTrigger(ctx.input, ctx.cursorOffset, ctx.surface);
  const items: IntelliSenseItem[] = [];
  const hasFailedBlock = Boolean(
    ctx.termBridge?.blocks.some((b) => b.status === 'Failed' || (b.exit_code ?? 0) !== 0),
  );
  const isNuContext =
    ctx.shellMode === 'nu' || trigger.kind === 'nu' || trigger.kind === 'pipe';

  // 1. CLI CommandCatalog suggestions (`discoverySuggest`)
  if (
    trigger.kind === 'vox' ||
    trigger.kind === 'general' ||
    trigger.kind === 'slash' ||
    trigger.kind === 'nu' ||
    trigger.kind === 'pipe'
  ) {
    for (const s of ctx.catalogSuggestions ?? []) {
      const fullCmd =
        s.completion.startsWith('/') || s.completion.startsWith('!')
          ? s.completion
          : `vox ${s.completion}`;
      const cat: CompletionCategory =
        s.category === 'slash'
          ? 'slash'
          : s.category === 'nu'
            ? 'nu'
            : 'command';
      const badge =
        cat === 'slash' ? 'Slash' : cat === 'nu' ? 'Nu' : s.kind === 'flag' ? 'Flag' : 'CLI';
      const ghost = makeGhostSuffix(trigger.rawToken, fullCmd);
      items.push({
        id: `catalog-${s.action_id}-${s.completion}`,
        label: fullCmd,
        insertText: fullCmd,
        ghostSuffix: ghost,
        category: cat,
        badge,
        description: s.about,
        signature: s.signature || fullCmd,
        actionId: s.action_id,
        score: (s.score ?? 500) + (ghost ? 250 : 0),
      });
    }
  }

  // 2. Slash commands & Installed Skills (`buildSlashEntries` + Terminal `CommandRegistry`)
  if (
    trigger.kind === 'slash' ||
    trigger.kind === 'at_skill' ||
    trigger.kind === 'at_all' ||
    (ctx.forceOpen && !trigger.rawToken)
  ) {
    const slashEntries = buildSlashEntries(ctx.skills ?? []);
    const mergedSlash: Array<SlashEntry & { signature?: string }> = [...slashEntries];

    if (ctx.surface === 'terminal') {
      const seen = new Set(mergedSlash.map((s) => s.cmd));
      for (const t of TERMINAL_COMMAND_REGISTRY) {
        if (!seen.has(t.cmd)) {
          seen.add(t.cmd);
          mergedSlash.push({
            cmd: t.cmd,
            desc: t.desc,
            kind: 'builtin',
            icon: 'terminal',
            signature: t.usage,
          });
        }
      }
    }

    for (const entry of mergedSlash) {
      const isSkill = entry.kind === 'skill';
      if (trigger.kind === 'at_skill' && !isSkill) continue;
      if (trigger.kind === 'at_all' && !isSkill) continue;

      const label =
        trigger.kind === 'at_skill' || trigger.kind === 'at_all'
          ? `@skill:${entry.cmd.slice(1)}`
          : entry.cmd;
      const sc = fuzzyScore(
        trigger.query,
        `${entry.cmd.slice(1)} ${entry.desc}`,
      );
      if (trigger.query && sc === 0) continue;

      let boost = 0;
      if (hasFailedBlock && entry.cmd === '/audit') boost += 220;

      items.push({
        id: `slash-${label}`,
        label,
        insertText: label,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, label),
        category: isSkill ? 'skill' : 'slash',
        badge: isSkill ? 'Skill' : 'Slash',
        description: entry.desc,
        signature: entry.signature ?? entry.cmd,
        actionId: `slash.${entry.cmd.slice(1)}`,
        score: sc + boost,
        chip: isSkill
          ? {
              kind: 'skill',
              label: entry.cmd.slice(1),
              meta: 'skill',
            }
          : undefined,
        slashEntry: entry,
      });
    }
  }

  // 3. Live Terminal Blocks & `@term` options (`terminalBridge`)
  if (
    trigger.kind === 'at_term' ||
    trigger.kind === 'at_all' ||
    trigger.kind === 'prompt' ||
    (ctx.forceOpen && hasFailedBlock)
  ) {
    for (const opt of TERM_MENTION_OPTIONS) {
      const sc = fuzzyScore(trigger.query, `${opt.token.slice(1)} ${opt.label} ${opt.desc}`);
      if (trigger.query && sc === 0 && trigger.kind !== 'prompt') continue;
      const failedBoost = hasFailedBlock && opt.token === '@term:failed' ? 350 : 0;
      items.push({
        id: `term-opt-${opt.token}`,
        label: opt.token,
        insertText: opt.token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, opt.token),
        category: 'term',
        badge: 'Term',
        description: opt.desc,
        signature: opt.token,
        score: sc + 120 + failedBoost,
        chip: {
          kind: 'term',
          label: opt.token,
          meta: opt.desc,
        },
      });
    }

    const sessionId = ctx.termBridge?.activeSessionId ?? 'session';
    for (const block of ctx.termBridge?.blocks ?? []) {
      const token = `@term:${sessionId}#${block.id}`;
      const isFailed = block.status === 'Failed' || (block.exit_code ?? 0) !== 0;
      const desc = `${block.input} (${isFailed ? `exit ${block.exit_code ?? 1}` : 'ok'})`;
      const sc = fuzzyScore(trigger.query, `${token.slice(1)} ${block.input}`);
      if (trigger.query && sc === 0 && !ctx.forceOpen) continue;
      items.push({
        id: `term-block-${block.id}`,
        label: token,
        insertText: token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, token),
        category: 'term',
        badge: isFailed ? `Exit ${block.exit_code ?? 1}` : 'Term',
        description: desc,
        signature: `${token} — $ ${block.input}`,
        score: sc + (isFailed ? 400 : 140),
        chip: {
          kind: 'term',
          label: token,
          meta: isFailed ? `failed (${block.exit_code ?? 1})` : 'ok',
        },
      });
    }

    if (hasFailedBlock && (trigger.kind === 'prompt' || ctx.forceOpen)) {
      const fixCmd = '?fix error in last block';
      items.push({
        id: 'prompt-fix-last-block',
        label: fixCmd,
        insertText: fixCmd,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, fixCmd),
        category: 'term',
        badge: 'Recovery',
        description: 'Ask the agent to diagnose and repair the failed terminal block',
        signature: fixCmd,
        score: 920,
      });
    }
  }

  // 4. Nushell structured pipeline stages (`!nu` / `|`)
  if (
    trigger.kind === 'nu' ||
    trigger.kind === 'pipe' ||
    (ctx.surface === 'terminal' && isNuContext && trigger.kind === 'general')
  ) {
    for (const stage of NU_PIPELINE_STAGES) {
      const sc = fuzzyScore(trigger.query, `${stage.name} ${stage.signature} ${stage.desc}`);
      if (trigger.query && sc === 0) continue;
      const insertText =
        trigger.kind === 'pipe'
          ? `${ctx.input.slice(0, trigger.replaceStart)}${stage.name}`
          : trigger.kind === 'nu'
            ? `!nu ${stage.name}`
            : stage.name;
      const id = `nu-stage-${stage.name.replace(/\s+/g, '-')}`;
      if (items.some((i) => i.insertText === insertText)) continue;
      items.push({
        id,
        label: trigger.kind === 'pipe' ? `| ${stage.name}` : `!nu ${stage.name}`,
        insertText,
        ghostSuffix: makeGhostSuffix(
          trigger.kind === 'pipe' ? trigger.rawToken : ctx.input.trimStart(),
          trigger.kind === 'pipe' ? stage.name : insertText,
        ),
        category: 'nu',
        badge: 'Nu',
        description: stage.desc,
        signature: stage.signature,
        actionId: `nu.${stage.name.replace(/\s+/g, '_')}`,
        score: sc + (isNuContext ? 220 : 60),
      });
    }
  }

  // 5. Canonical MCP Tools from `CAPABILITY_INDEX` (superseding legacy vox-vscode mcpToolRegistry)
  if (trigger.kind === 'at_mcp' || trigger.kind === 'at_all') {
    const loadedIndex = getLoadedCapabilityIndex()?.CAPABILITY_INDEX;
    if (!loadedIndex) {
      void loadCapabilityIndex().catch(() => {});
    }
    const fallbackRow = (id: string, mcp: string, summary: string): CapabilityIndexEntry => ({
      id, kind: 'mcp_tool', title: mcp, summary, mcp,
      domain: null, cli: null, viewKey: null, crate: null, layer: null, docs: null, docOrigin: null,
      coverage: null, coverageReason: null, coverageAmbiguous: false, equivalents: [], refs: [], tier: null, status: null,
      sideEffect: null, lane: null, catalogForm: false, navLabel: null, source: null,
    });
    const fallbackMcpRows: CapabilityIndexEntry[] = [
      fallbackRow('mcp-vox-search-query', 'vox_search_query', 'Query hybrid lexical and semantic index across codebase and docs'),
      fallbackRow('mcp-vox-skill-search', 'vox_skill_search', 'Search and inspect registered agent skills'),
      fallbackRow('mcp-vox-kb-query', 'vox_kb_query', 'Query verified knowledge base facts and taxonomy'),
      fallbackRow('mcp-vox-run-shell', 'vox_run_shell', 'Execute verified commands with security level check'),
    ];
    const mcpRows = (loadedIndex ?? fallbackMcpRows).filter((r: CapabilityIndexEntry) => r.kind === 'mcp_tool' || r.mcp != null);
    for (const row of mcpRows) {
      const toolName = row.mcp ?? row.title;
      const sc = fuzzyScore(trigger.query, `${toolName} ${row.summary ?? ''}`);
      if (trigger.query && sc === 0) continue;
      const token = `@mcp:${toolName}`;
      items.push({
        id: `mcp-${row.id}`,
        label: token,
        insertText: token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, token),
        category: 'mcp',
        badge: 'MCP',
        description: row.summary ?? row.title,
        signature: `${toolName} (${row.domain ?? 'platform'})`,
        score: sc + 80,
        chip: {
          kind: 'skill',
          label: token,
          meta: row.domain ?? 'mcp',
        },
      });
      if (items.filter((i) => i.category === 'mcp').length >= 12) break;
    }
  }

  // 6. Workspace Files & Active Agents (`@file`, `@agent`)
  if (trigger.kind === 'at_file' || trigger.kind === 'at_all') {
    for (const agent of ctx.agents ?? []) {
      const sc = fuzzyScore(trigger.query, `${agent.id} ${agent.codename ?? ''}`);
      if (trigger.query && sc === 0) continue;
      const token = `@${agent.id}`;
      items.push({
        id: `agent-${agent.id}`,
        label: token,
        insertText: token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, token),
        category: 'file',
        badge: 'Agent',
        description: `${agent.codename ?? 'Agent'} · ${agent.phase ?? 'ready'}`,
        signature: token,
        score: sc + 160,
        chip: {
          kind: 'agent',
          label: agent.codename ? `${agent.id} · ${agent.codename}` : agent.id,
          meta: agent.phase,
        },
      });
    }

    for (const filePath of ctx.repoFiles ?? []) {
      const sc = fuzzyScore(trigger.query, filePath);
      if (trigger.query && sc === 0) continue;
      const token = `@${filePath}`;
      items.push({
        id: `file-${filePath}`,
        label: token,
        insertText: token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, token),
        category: 'file',
        badge: 'File',
        description: `Attach workspace file ${filePath}`,
        signature: filePath,
        score: sc + 130,
        chip: {
          kind: 'file',
          label: filePath,
        },
      });
    }
  }

  // 7. `.vox` Language Surface Symbols & Decorators (`vox-lsp` parity)
  if (trigger.kind === 'at_symbol' || trigger.kind === 'at_all') {
    for (const sym of VOX_LSP_SYMBOLS) {
      const cleanName = sym.name.replace(/^@/, '');
      const sc = fuzzyScore(trigger.query, `${cleanName} ${sym.desc}`);
      if (trigger.query && sc === 0) continue;
      const token = `@symbol:${cleanName}`;
      items.push({
        id: `symbol-${cleanName}`,
        label: token,
        insertText: token,
        ghostSuffix: makeGhostSuffix(trigger.rawToken, token),
        category: 'symbol',
        badge: 'LSP',
        description: sym.desc,
        signature: sym.signature,
        score: sc + 95,
        chip: {
          kind: 'file',
          label: sym.name,
          meta: 'vox-lsp',
        },
      });
    }
  }

  return items.sort((a, b) => b.score - a.score || a.label.localeCompare(b.label));
}
