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
const STATUS_BADGE = {
  experimental: { text: 'Experimental', variant: 'caution' },
  research:     { text: 'Research',     variant: 'note'    },
  roadmap:      { text: 'Roadmap',      variant: 'note'    },
  deprecated:   { text: 'Deprecated',   variant: 'danger'  },
  legacy:       { text: 'Legacy',       variant: 'tip'     },
};

// Directories under docs/src/ that should never appear in the sidebar
const EXCLUDED_DIRS = new Set(['archive', '.well-known']);

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
        // Derive Starlight document slug from relative path under docs/src
        const slug = docSlug(relative(root, full));
        const rel = slug ? slug : '/';
        const relRepo = relative(repoRoot, full).replace(/\\/g, '/');
        const ledgerEntry = ledger?.pages ? (ledger.pages[relRepo] || ledger.pages[relRepo.replace(/\.mdx?$/, '.md')]) : null;
        pages.push({
          title:          data.title || entry.replace(/\.mdx?$/, ''),
          link:           rel,
          category:       data.category || null,
          sort_order:     data.sort_order ?? 999,
          status:         data.status || 'current',
          freshnessState: ledgerEntry?.state || 'unanchored',
        });
      } catch {
        // skip unreadable / non-frontmatter files
      }
    }
  }
  return pages;
}

function makeItem(p, isMain = true) {
  let badge = STATUS_BADGE[p.status];
  if (!badge && isMain && p.link !== '/' && p.freshnessState) {
    if (p.freshnessState === 'drifting') {
      badge = { text: 'Drifting', variant: 'caution' };
    } else if (p.freshnessState === 'broken' || p.freshnessState === 'unanchored') {
      badge = { text: 'Unverified', variant: 'note' };
    }
  }
  return badge
    ? { label: p.title, link: p.link, badge }
    : { label: p.title, link: p.link };
}

export function getSidebar() {
  const repoRoot = DEFAULT_REPO_ROOT;
  const docsSrc = join(repoRoot, 'docs', 'src');

  const ledger = loadFreshnessLedger();
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
    items.sort(sortFn);
    sidebar.push({
      label: section,
      items: items.map((p) => makeItem(p, true)),
      collapsed: COLLAPSED_SECTIONS.has(section),
    });
    grouped.delete(section);
  }

  // Any category not in SECTION_ORDER appended at end (collapsed — catch-all for new sections)
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
