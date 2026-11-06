import { readFileSync, readdirSync, statSync } from 'node:fs';
import { join, relative, extname } from 'node:path';
import matter from 'gray-matter';
import { isInternalsStatus } from './page-status.mjs';
import { docSlug } from './doc-slug.mjs';
import { loadFreshnessLedger } from './docs-freshness.mjs';
import { DEFAULT_REPO_ROOT } from './repo-mounts.mjs';

// SSOT: contracts/documentation/docs-sidebar-section-order.v1.json
const __sidebarOrderPath = join(
  DEFAULT_REPO_ROOT,
  'contracts/documentation/docs-sidebar-section-order.v1.json'
);
const __sidebarOrder = JSON.parse(readFileSync(__sidebarOrderPath, 'utf8'));
const SECTION_ORDER = __sidebarOrder.sections;
const COLLAPSED_SECTIONS = new Set(__sidebarOrder.collapsed_sections ?? []);

// Status values that earn a sidebar badge so users know a page is not yet stable.
// Normal/Verified/Unanchored pages do NOT emit sidebar badges to prevent badge spam.
const STATUS_BADGE = {
  experimental: { text: 'Experimental', variant: 'caution' },
  research:     { text: 'Research',     variant: 'note'    },
  roadmap:      { text: 'Roadmap',      variant: 'note'    },
  deprecated:   { text: 'Deprecated',   variant: 'danger'  },
  legacy:       { text: 'Legacy',       variant: 'tip'     },
};

const LAYER_LABELS = {
  0: 'L0 — Pure Types & Contracts',
  1: 'L1 — Primitives & Crypto',
  2: 'L2 — Domain & Graph Engine',
  3: 'L3 — Compiler, DB & Agents',
  4: 'L4 — Native Plugins & TUI',
  5: 'L5 — CLI, GUI & Daemon',
};

const DOMAIN_GROUP_ORDER = [
  { key: 'language',      label: 'Language & Compiler' },
  { key: 'runtime',       label: 'Runtime & Actors' },
  { key: 'orchestration', label: 'Orchestration & Agents' },
  { key: 'data',          label: 'Data, VoxDb & Turso' },
  { key: 'ml',            label: 'ML, MENS & Speech' },
  { key: 'gui',           label: 'GUI & Axis Workbench' },
  { key: 'security',      label: 'Security & Clavis Secrets' },
  { key: 'mesh',          label: 'Mesh & Populi' },
  { key: 'platform',      label: 'Platform, Tooling & CI' },
  { key: 'documentation', label: 'Documentation & Governance' },
];

// Curated high-value Architecture pages users look for first (Pinned at top of Architecture SSOTs)
const PINNED_ARCHITECTURE_LINKS = new Map([
  ['architecture/where-things-live',                    { order: 1, label: 'Where Things Live (Crate Map)' }],
  ['architecture/subsystems/index.generated',           { order: 2, label: 'Subsystem Atlas (L0–L5)' }],
  ['architecture/subsystems',                           { order: 2, label: 'Subsystem Atlas (L0–L5)' }],
  ['architecture/agentos-ssot-2026',                    { order: 3, label: 'AgentOS & Orchestrator SSOT' }],
  ['architecture/cryptography-ssot-2026',               { order: 4, label: 'Cryptography & Clavis Secrets' }],
  ['architecture/telemetry-trust-ssot',                 { order: 5, label: 'Telemetry & Trust Boundaries' }],
  ['architecture/openwiki-self-healing-docs-ssot-2026', { order: 6, label: 'Self-Maintaining Docs SSOT' }],
]);

// Directories under docs/src/ that should never appear in the sidebar
const EXCLUDED_DIRS = new Set(['archive', '.well-known']);

function parseCrateLayers(repoRoot) {
  const map = new Map();
  try {
    const raw = readFileSync(join(repoRoot, 'docs/src/architecture/layers.toml'), 'utf8');
    let inCrates = false;
    for (const rawLine of raw.split(/\r?\n/)) {
      const line = rawLine.split('#')[0].trim();
      if (!line) continue;
      if (line.startsWith('[')) {
        inCrates = line === '[crates]';
        continue;
      }
      if (!inCrates) continue;
      const eqIdx = line.indexOf('=');
      if (eqIdx < 0) continue;
      const crateName = line.slice(0, eqIdx).trim();
      const rhs = line.slice(eqIdx + 1);
      const layerMatch = rhs.match(/layer\s*=\s*(\d+)/);
      if (layerMatch) {
        map.set(crateName, Number(layerMatch[1]));
      }
    }
  } catch {
    // fallback if layers.toml unavailable
  }
  return map;
}

function cleanSidebarTitle(title, link) {
  let cleaned = String(title || '').trim();
  if (link && link.startsWith('api/') && link !== 'api/index.generated') {
    cleaned = cleaned.replace(/\s+API Reference\s*\(generated\)$/i, '');
  }
  cleaned = cleaned.replace(/\s*\(Generated\)$/i, '');
  return cleaned;
}

function inferDomainGroup(page) {
  const d = String(page.domain || '').toLowerCase().trim();
  const t = String(page.topic || '').toLowerCase().trim();
  const slug = String(page.link || '').toLowerCase();

  if (d === 'language' || t.includes('grammar') || t.includes('compiler') || t.includes('type') || slug.includes('compiler') || slug.includes('grammar') || slug.includes('syntax') || slug.includes('codegen')) {
    return 'language';
  }
  if (d === 'runtime' || t.includes('actor') || t.includes('workflow') || slug.includes('actor') || slug.includes('workflow') || slug.includes('runtime')) {
    return 'runtime';
  }
  if (d === 'orchestration' || d === 'agentos' || t.includes('orchestrat') || t.includes('mcp') || t.includes('skill') || slug.includes('agent') || slug.includes('orchestrat') || slug.includes('mcp') || slug.includes('skill') || slug.includes('harness')) {
    return 'orchestration';
  }
  if (d === 'data' || t.includes('database') || t.includes('search') || slug.includes('db') || slug.includes('turso') || slug.includes('codex') || slug.includes('search') || slug.includes('tantivy')) {
    return 'data';
  }
  if (d === 'ml' || t.includes('mens') || t.includes('oratio') || slug.includes('mens') || slug.includes('oratio') || slug.includes('speech') || slug.includes('qlora') || slug.includes('train')) {
    return 'ml';
  }
  if (d === 'gui' || t.includes('gui') || slug.includes('gui') || slug.includes('axis') || slug.includes('loquela') || slug.includes('webview') || slug.includes('svelte') || slug.includes('react')) {
    return 'gui';
  }
  if (d === 'security' || t.includes('secret') || t.includes('crypto') || slug.includes('crypto') || slug.includes('clavis') || slug.includes('secret') || slug.includes('trust') || slug.includes('auth')) {
    return 'security';
  }
  if (d === 'mesh' || t.includes('populi') || t.includes('mesh') || slug.includes('populi') || slug.includes('mesh') || slug.includes('a2a') || slug.includes('iroh')) {
    return 'mesh';
  }
  if (d === 'documentation' || t.includes('doc-') || slug.includes('doc') || slug.includes('scientia') || slug.includes('kb') || slug.includes('wiki')) {
    return 'documentation';
  }
  return 'platform';
}

function collectPages(dir, root, repoRoot, ledger) {
  const pages = [];
  let entries;
  try {
    entries = readdirSync(dir);
  } catch {
    return pages;
  }
  for (const entry of entries) {
    const full = join(dir, entry);
    let stat;
    try {
      stat = statSync(full);
    } catch {
      continue;
    }
    if (stat.isDirectory()) {
      if (EXCLUDED_DIRS.has(entry)) continue;
      pages.push(...collectPages(full, root, repoRoot, ledger));
    } else if (extname(entry) === '.md' || extname(entry) === '.mdx') {
      try {
        const raw = readFileSync(full, 'utf8');
        const { data } = matter(raw);
        const slug = docSlug(relative(root, full));
        const rel = slug ? slug : '/';
        const relRepo = relative(repoRoot, full).replace(/\\/g, '/');
        const ledgerEntry = ledger?.pages ? (ledger.pages[relRepo] || ledger.pages[relRepo.replace(/\.mdx?$/, '.md')]) : null;
        const isGeneratedOrAnchored =
          entry.endsWith('.generated.md') ||
          (Array.isArray(data.code_anchors) && data.code_anchors.length > 0);
        pages.push({
          title:          cleanSidebarTitle(data.title || entry.replace(/\.mdx?$/, ''), rel),
          link:           rel,
          fileBase:       entry.replace(/\.mdx?$/, ''),
          isGenerated:    entry.endsWith('.generated.md'),
          category:       data.category || null,
          domain:         data.domain || null,
          topic:          data.topic || null,
          sort_order:     data.sort_order ?? 999,
          status:         data.status || 'current',
          freshnessState: ledgerEntry?.state || (isGeneratedOrAnchored ? 'verified' : 'unanchored'),
        });
      } catch {
        // skip unreadable / non-frontmatter files
      }
    }
  }
  return pages;
}

function makeItem(p, isMain = true, customLabel = null) {
  let badge = STATUS_BADGE[p.status];
  // Only show a sidebar badge for genuinely drifting or broken pages — never spam "Verified" or "Unverified"
  if (!badge && isMain && p.link !== '/' && (p.freshnessState === 'drifting' || p.freshnessState === 'broken')) {
    badge = { text: 'Drifting', variant: 'caution' };
  }
  const label = customLabel || p.title;
  return badge
    ? { label, link: p.link, badge }
    : { label, link: p.link };
}

function buildApiCratesSection(items, crateLayers, sortFn) {
  items.sort(sortFn);
  const overviewItems = [];
  const byLayer = new Map([[0, []], [1, []], [2, []], [3, []], [4, []], [5, []]]);

  for (const p of items) {
    if (p.link === 'api/index.generated' || p.fileBase === 'index.generated') {
      overviewItems.push(makeItem(p, true, 'Crate API Overview & Index'));
      continue;
    }
    const crateName = p.fileBase.replace(/\.generated$/, '');
    const layer = crateLayers.get(crateName) ?? 3;
    const bucket = byLayer.get(layer) ?? byLayer.get(3);
    bucket.push(makeItem(p, true));
  }

  const subgroups = [...overviewItems];
  for (const [layer, layerItems] of byLayer.entries()) {
    if (layerItems.length === 0) continue;
    subgroups.push({
      label: `${LAYER_LABELS[layer]} (${layerItems.length})`,
      collapsed: true,
      items: layerItems,
    });
  }
  return subgroups;
}

function buildArchitectureSsotSection(items, sortFn) {
  const pinned = [];
  const layerHubs = [];
  const byDomain = new Map(DOMAIN_GROUP_ORDER.map((g) => [g.key, []]));

  for (const p of items) {
    if (PINNED_ARCHITECTURE_LINKS.has(p.link)) {
      const spec = PINNED_ARCHITECTURE_LINKS.get(p.link);
      pinned.push({ order: spec.order, item: makeItem(p, true, spec.label) });
      continue;
    }
    if (p.link.startsWith('architecture/subsystems/')) {
      layerHubs.push(p);
      continue;
    }
    const domKey = inferDomainGroup(p);
    const bucket = byDomain.get(domKey) || byDomain.get('platform');
    bucket.push(p);
  }

  pinned.sort((a, b) => a.order - b.order);
  layerHubs.sort(sortFn);

  const result = [];
  if (pinned.length > 0) {
    result.push({
      label: 'Pinned — Core Architecture',
      collapsed: false,
      items: pinned.map((entry) => entry.item),
    });
  }
  if (layerHubs.length > 0) {
    result.push({
      label: 'Subsystem Layer Atlas (L0–L5)',
      collapsed: true,
      items: layerHubs.map((p) => makeItem(p, true)),
    });
  }
  for (const { key, label } of DOMAIN_GROUP_ORDER) {
    const groupPages = byDomain.get(key) || [];
    if (groupPages.length === 0) continue;
    groupPages.sort(sortFn);
    result.push({
      label: `${label} (${groupPages.length})`,
      collapsed: true,
      items: groupPages.map((p) => makeItem(p, true)),
    });
  }
  return result;
}

function buildLanguageReferenceSection(items, sortFn) {
  items.sort(sortFn);
  const coreLang = [];
  const generatedTables = [];
  const mensAndAi = [];
  const contractsAndOps = [];
  const redirects = [];

  for (const p of items) {
    const titleLower = p.title.toLowerCase();
    const linkLower = p.link.toLowerCase();
    if (titleLower.includes('(redirect)') || titleLower.includes('legacy path')) {
      redirects.push(makeItem(p, true));
    } else if (p.isGenerated) {
      generatedTables.push(makeItem(p, true));
    } else if (
      linkLower.includes('ref-') ||
      titleLower.includes('syntax') ||
      titleLower.includes('type system') ||
      titleLower.includes('standard library') ||
      titleLower.includes('built-in') ||
      titleLower.includes('operator precedence') ||
      titleLower.includes('database') ||
      titleLower.includes('token system') ||
      titleLower.includes('language ergonomics') ||
      titleLower.includes('system prompt')
    ) {
      coreLang.push(makeItem(p, true));
    } else if (
      titleLower.includes('mens') ||
      titleLower.includes('oratio') ||
      titleLower.includes('speech') ||
      titleLower.includes('socrates') ||
      titleLower.includes('scientia') ||
      titleLower.includes('populi') ||
      titleLower.includes('mesh')
    ) {
      mensAndAi.push(makeItem(p, true));
    } else {
      contractsAndOps.push(makeItem(p, true));
    }
  }

  const groups = [];
  if (coreLang.length > 0) {
    groups.push({
      label: `Core .vox Language & Stdlib (${coreLang.length})`,
      collapsed: false,
      items: coreLang,
    });
  }
  if (generatedTables.length > 0) {
    groups.push({
      label: `CLI, MCP & Config Tables (${generatedTables.length})`,
      collapsed: true,
      items: generatedTables,
    });
  }
  if (mensAndAi.length > 0) {
    groups.push({
      label: `MENS, Speech & Mesh Reference (${mensAndAi.length})`,
      collapsed: true,
      items: mensAndAi,
    });
  }
  if (contractsAndOps.length > 0) {
    groups.push({
      label: `Platform, Security & Contracts (${contractsAndOps.length})`,
      collapsed: true,
      items: contractsAndOps,
    });
  }
  if (redirects.length > 0) {
    groups.push({
      label: `Compatibility Redirects (${redirects.length})`,
      collapsed: true,
      items: redirects,
    });
  }
  return groups;
}

export function getSidebar() {
  const repoRoot = DEFAULT_REPO_ROOT;
  const docsSrc = join(repoRoot, 'docs', 'src');

  const ledger = loadFreshnessLedger();
  const crateLayers = parseCrateLayers(repoRoot);
  const allPages = collectPages(docsSrc, docsSrc, repoRoot, ledger);

  const mainPages = [];
  const internalsPages = [];
  for (const page of allPages) {
    if (isInternalsStatus(page.status)) {
      internalsPages.push(page);
    } else {
      mainPages.push(page);
    }
  }

  const grouped = new Map();
  const rootItems = [];

  for (const page of mainPages) {
    if (!page.category) {
      rootItems.push(page);
    } else {
      if (!grouped.has(page.category)) grouped.set(page.category, []);
      grouped.get(page.category).push(page);
    }
  }

  const sortFn = (a, b) =>
    a.sort_order - b.sort_order || a.title.localeCompare(b.title);

  const sidebar = [];

  rootItems.sort(sortFn);
  for (const p of rootItems) {
    sidebar.push(makeItem(p, true));
  }

  for (const section of SECTION_ORDER) {
    const items = grouped.get(section);
    if (!items || items.length === 0) continue;

    let sectionItems;
    if (section === 'API Reference — Crates') {
      sectionItems = buildApiCratesSection(items, crateLayers, sortFn);
    } else if (section === 'Architecture SSOTs') {
      sectionItems = buildArchitectureSsotSection(items, sortFn);
    } else if (section === 'Language Reference') {
      sectionItems = buildLanguageReferenceSection(items, sortFn);
    } else {
      items.sort(sortFn);
      sectionItems = items.map((p) => makeItem(p, true));
    }

    sidebar.push({
      label: section,
      items: sectionItems,
      collapsed: COLLAPSED_SECTIONS.has(section),
    });
    grouped.delete(section);
  }

  for (const [section, items] of grouped) {
    items.sort(sortFn);
    sidebar.push({
      label: section,
      items: items.map((p) => makeItem(p, true)),
      collapsed: true,
    });
  }

  if (internalsPages.length > 0) {
    const internalsRootItems = [];
    const internalsGrouped = new Map();
    for (const page of internalsPages) {
      if (!page.category) {
        internalsRootItems.push(page);
      } else {
        if (!internalsGrouped.has(page.category)) internalsGrouped.set(page.category, []);
        internalsGrouped.get(page.category).push(page);
      }
    }

    internalsRootItems.sort(sortFn);

    const internalsSubgroups = [];
    for (const section of SECTION_ORDER) {
      const items = internalsGrouped.get(section);
      if (!items || items.length === 0) continue;
      items.sort(sortFn);
      internalsSubgroups.push({
        label: section,
        items: items.map((p) => makeItem(p, false)),
        collapsed: true,
      });
      internalsGrouped.delete(section);
    }

    const remainingCategories = Array.from(internalsGrouped.keys()).sort((a, b) => a.localeCompare(b));
    for (const cat of remainingCategories) {
      const items = internalsGrouped.get(cat);
      if (!items || items.length === 0) continue;
      items.sort(sortFn);
      internalsSubgroups.push({
        label: cat,
        items: items.map((p) => makeItem(p, false)),
        collapsed: true,
      });
    }

    sidebar.push({
      label: 'Internals',
      collapsed: true,
      items: [
        ...internalsRootItems.map((p) => makeItem(p, false)),
        ...internalsSubgroups,
      ],
    });
  }

  return sidebar;
}
