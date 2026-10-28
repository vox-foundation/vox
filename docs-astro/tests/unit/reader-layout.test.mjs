import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const CSS_PATH = resolve(fileURLToPath(import.meta.url), '../../../src/styles/reader-layout.css');

test('reader-layout.css exists and contains expected rules', () => {
  assert.ok(existsSync(CSS_PATH), 'reader-layout.css must exist');
  const css = readFileSync(CSS_PATH, 'utf8');

  // Article width expansion rules
  assert.match(css, /--sl-content-width:\s*60rem;/, 'Must define 60rem content width for single sidebar collapse');
  assert.match(css, /--sl-content-width:\s*74rem;/, 'Must define 74rem content width for both sidebars collapse');

  // Typography line length constraint
  assert.match(css, /max-width:\s*75ch;/, 'Must constrain prose paragraphs/lists to 75ch');

  // Hover and touch device discrimination
  assert.match(css, /@media\s*\(\s*hover:\s*hover\s*\)/, 'Must contain hover: hover query for overlays');
  assert.match(css, /@media\s*\(\s*hover:\s*none\s*\)/, 'Must contain hover: none guard for touch devices');

  // Reduced motion preference
  assert.match(css, /@media\s*\(\s*prefers-reduced-motion:\s*reduce\s*\)/, 'Must honor reduced motion preferences');

  // Overflow-x containment
  assert.match(css, /overflow-x:\s*hidden;/, 'Must guard viewport overflow-x');

  // Hideable left & right sidebar buttons and states
  assert.match(css, /#vox-sidebar-hide-btn/, 'Must define themed left sidebar hide button');
  assert.match(css, /#vox-sidebar-restore-btn/, 'Must define themed left sidebar restore pill');
  assert.match(css, /html\[data-sidebar-state="hidden"\]/, 'Must define hidden left sidebar state');
  assert.match(css, /#vox-toc-hide-btn/, 'Must define themed right TOC hide button');
  assert.match(css, /#vox-toc-restore-btn/, 'Must define themed right TOC restore pill');
});
