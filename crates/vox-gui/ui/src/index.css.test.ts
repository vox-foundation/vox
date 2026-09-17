// src/index.css.test.ts
import { describe, it, expect } from 'vitest';
import { readFileSync } from 'fs';
import { resolve } from 'path';
import { STATUS_TONE } from './styles/tokens';

const css = readFileSync(resolve(__dirname, './index.css'), 'utf8');

describe('index.css global a11y rules', () => {
  it('contains a :focus-visible rule', () => {
    expect(css).toContain(':focus-visible');
  });

  it('uses --color-accent-default for the focus ring color', () => {
    // The ring must reference the token, not a hardcoded hex.
    expect(css).toContain('--color-accent-default');
  });

  it('contains a prefers-reduced-motion block that targets vox-* animations', () => {
    expect(css).toContain('prefers-reduced-motion');
    expect(css).toContain('animation');
  });

  it('sets color-scheme per theme so native controls (e.g. <select> option popups) match the app theme instead of defaulting to light-mode white-on-white', () => {
    expect(css).toMatch(/:root\s*\{[^}]*color-scheme:\s*dark/);
    expect(css).toMatch(/\[data-theme="travertine"\]\s*\{[^}]*color-scheme:\s*light/);
  });
});


describe('STATUS_TONE', () => {
  it('contains the mandatory keys', () => {
    expect(STATUS_TONE.pass).toBeDefined();
    expect(STATUS_TONE.fail).toBeDefined();
    expect(STATUS_TONE.warn).toBeDefined();
    expect(STATUS_TONE.Executing).toBeDefined();
  });
});

describe('ds-section-head (B7: migrated from ds/components.css so it actually loads)', () => {
  it('is defined in the app stylesheet', () => {
    expect(css).toContain('.ds-section-head');
  });

  it('underlines the heading (divider below, never a cap above)', () => {
    const start = css.indexOf('.ds-section-head');
    const body = css.slice(start, css.indexOf('}', start));
    expect(body).toContain('border-bottom: 1px solid var(--color-border-subtle)');
    expect(body).toContain('font-family: var(--font-family-display)');
    expect(body).not.toContain('border-top');
  });

  it('does not pull in the standalone ds bundle (token/font double-load hazard)', () => {
    expect(css).not.toContain("@import '../ds");
    expect(css).not.toContain('ds/styles.css');
  });
});

describe('theme leftovers (dock chrome + range thumb)', () => {
  const dockCss = readFileSync(resolve(__dirname, './styles/dockview-vox.css'), 'utf8');

  it('tokenizes dockview tab foreground (no zinc-100 rgb)', () => {
    expect(dockCss).not.toContain('rgb(244, 244, 245)');
  });

  it('tokenizes vox-range thumb (no hardcoded white)', () => {
    const start = css.indexOf('@utility vox-range');
    expect(start).toBeGreaterThan(-1);
    const next = css.indexOf('@utility', start + 1);
    const body = css.slice(start, next === -1 ? undefined : next);
    expect(body).not.toContain('background: #fff');
  });
});

describe('overline (theme: rule under the text, never a Latin cap above)', () => {
  it('redefines Tailwind overline as an underline with offset', () => {
    const start = css.indexOf('@utility overline');
    expect(start).toBeGreaterThan(-1);
    const body = css.slice(start, css.indexOf('}', start));
    expect(body).toContain('text-decoration-line: underline');
    expect(body).toContain('text-underline-offset');
    expect(body).not.toContain('text-decoration-line: overline');
  });
});

