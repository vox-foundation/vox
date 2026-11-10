import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const CSS_PATH = resolve(fileURLToPath(import.meta.url), '../../../src/styles/reader-layout.css');

test('reader-layout.css exists and contains expected rules', () => {
  assert.ok(existsSync(CSS_PATH), 'reader-layout.css must exist');
  const css = readFileSync(CSS_PATH, 'utf8');

  // Article width expansion rules (recover space when left, right, or both panes are closed)
  assert.match(css, /--sl-content-width:\s*calc\(100vw\s*-\s*23rem\)/, 'Must expand content width when left sidebar is closed');
  assert.match(css, /--sl-content-width:\s*calc\(100vw\s*-\s*var\(--sl-sidebar-width\)\s*-\s*4\.5rem\)/, 'Must expand content width when right TOC is closed');
  assert.match(css, /--sl-content-width:\s*calc\(100vw\s*-\s*6rem\)/, 'Must expand content width when both panes are closed');
  assert.match(css, /flex:\s*0\s+0\s+0px\s*!important;/, 'Must collapse right-sidebar-container flex-basis to 0px when hidden');
  assert.match(css, /--sl-content-margin-inline:\s*0\s+auto;/, 'Must anchor .sl-container to start edge of .content-panel');

  // Prose & lists fill the active --sl-content-width container
  assert.match(css, /max-width:\s*100%;/, 'Must allow prose paragraphs/lists to fill expanded --sl-content-width');

  // Hover and touch device discrimination
  assert.match(css, /@media\s*\(\s*hover:\s*hover\s*\)/, 'Must contain hover: hover query for overlays');
  assert.match(css, /@media\s*\(\s*hover:\s*none\s*\)/, 'Must contain hover: none guard for touch devices');

  // Reduced motion preference
  assert.match(css, /@media\s*\(\s*prefers-reduced-motion:\s*reduce\s*\)/, 'Must honor reduced motion preferences');

  // Front page states its headline once: the hero's <h1>, not Starlight's title row too
  assert.match(css, /main:has\(\.vox-home-hero\)[^{]*h1#_top\)\s*\{\s*display:\s*none;/, 'Must hide the page title on the hero page');

  // Overflow-x containment
  assert.match(css, /overflow-x:\s*hidden;/, 'Must guard viewport overflow-x');

  // One seam chevron per pane, and a readable outline column
  assert.match(css, /#vox-sidebar-pin-toggle/, 'Must define the left seam chevron');
  assert.match(css, /#vox-toc-edge/, 'Must define the right seam chevron');
  assert.match(css, /html\[data-sidebar-state="hidden"\]/, 'Must define hidden left sidebar state');
  assert.match(css, /min-width:\s*16rem;/, 'Must keep the outline column at least 16rem');
  assert.match(css, /overflow-wrap:\s*break-word;/, 'Must break outline links on word boundaries');
  assert.match(css, /top:\s*var\(--sl-nav-height\)\s*!important;/, 'Must start right-sidebar below header navbar so header controls remain clickable');
  assert.match(css, /:root\[data-theme='light'\]\s+\.vox-tier-card--cyan/, 'Must define light-theme contrast surface for onboarding cards');
});
