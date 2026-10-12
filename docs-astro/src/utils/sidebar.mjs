import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { listDocPages } from './page-index.mjs';
import { isInternalsStatus } from './page-status.mjs';

// SSOT: contracts/documentation/docs-sidebar-section-order.v1.json
const __sidebarUtilsFile = fileURLToPath(import.meta.url);
const __repoRootSidebarOrder = join(__sidebarUtilsFile, '..', '..', '..', '..');
const __sidebarOrderPath = join(
  __repoRootSidebarOrder,
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

function makeItem(p) {
  const badge = STATUS_BADGE[p.status];
  return badge
    ? { label: p.title, link: p.link, badge }
    : { label: p.title, link: p.link };
}

const sortFn = (a, b) =>
  a.sort_order - b.sort_order || a.title.localeCompare(b.title);

/** Categories in SECTION_ORDER, then the rest (first-seen order, or alphabetical). */
function orderedCategories(grouped, { alphabetical = false } = {}) {
  const extra = [...grouped.keys()].filter((c) => !SECTION_ORDER.includes(c));
  if (alphabetical) extra.sort();
  return [...SECTION_ORDER.filter((c) => grouped.has(c)), ...extra];
}

function groupByCategory(pages) {
  const grouped = new Map();
  const rootItems = [];
  for (const page of pages) {
    if (!page.category) {
      rootItems.push(page);
    } else {
      if (!grouped.has(page.category)) grouped.set(page.category, []);
      grouped.get(page.category).push(page);
    }
  }
  rootItems.sort(sortFn);
  for (const items of grouped.values()) items.sort(sortFn);
  return { rootItems, grouped };
}

export function getSidebar() {
  // thisFile = <repo>/docs-astro/src/utils/sidebar.mjs
  // go up: utils → src → docs-astro → repo-root
  const thisFile = fileURLToPath(import.meta.url);
  const repoRoot = join(thisFile, '..', '..', '..', '..');
  const docsSrc = join(repoRoot, 'docs', 'src');

  // `.md` only: the `.mdx` home page is not a sidebar entry. Links are route
  // ids, which differ from the file path for names like `qwen-3.7` or `README`.
  const pages = listDocPages(docsSrc)
    .filter((page) => page.relPath.endsWith('.md'))
    .map((page) => ({ ...page, link: page.id }));

  // research/roadmap pages live only in the trailing Internals group.
  const main = groupByCategory(pages.filter((page) => !isInternalsStatus(page.status)));
  const internals = groupByCategory(pages.filter((page) => isInternalsStatus(page.status)));

  const sidebar = main.rootItems.map(makeItem);

  // Categories not in SECTION_ORDER come last, collapsed (catch-all for new sections).
  for (const section of orderedCategories(main.grouped)) {
    sidebar.push({
      label: section,
      items: main.grouped.get(section).map(makeItem),
      collapsed: !SECTION_ORDER.includes(section) || COLLAPSED_SECTIONS.has(section),
    });
  }

  const internalsItems = [
    ...internals.rootItems.map(makeItem),
    ...orderedCategories(internals.grouped, { alphabetical: true }).map((section) => ({
      label: section,
      items: internals.grouped.get(section).map(makeItem),
      collapsed: true,
    })),
  ];
  if (internalsItems.length > 0) {
    sidebar.push({ label: 'Internals', items: internalsItems, collapsed: true });
  }

  return sidebar;
}
