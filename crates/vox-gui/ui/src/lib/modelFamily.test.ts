import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';
import { familyKey, VERSIONED_CLOUD_MODEL_ID } from './modelFamily';

// src/lib -> src -> ui -> vox-gui -> crates -> repo root
const here = dirname(fileURLToPath(import.meta.url));
const BOOTSTRAP = resolve(here, '../../../../../contracts/orchestration/model-catalog.bootstrap.v1.json');
const catalogIds = (): string[] =>
  (JSON.parse(readFileSync(BOOTSTRAP, 'utf8')) as Array<{ id: string }>).map((e) => e.id);

describe('familyKey (mirror of the Rust models::family::family_key)', () => {
  it('drops version numbers and v-markers across naming shapes', () => {
    expect(familyKey('acme/widget-pro-4.8')).toBe('acme/widget-pro');
    expect(familyKey('acme/widget-v4.1-flash')).toBe('acme/widget-flash');
    expect(familyKey('acme/widget-v4-flash-0731')).toBe('acme/widget-flash');
    expect(familyKey('acme/gizmo-6-luna')).toBe('acme/gizmo-luna');
    expect(familyKey('Acme/Gizmo-Ultra-5.5')).toBe('acme/gizmo-ultra');
  });

  it('drops release-stage qualifiers and keeps the other words', () => {
    expect(familyKey('acme/gizmo-3.1-flash-lite-preview')).toBe('acme/gizmo-flash-lite');
    expect(familyKey('acme/gizmo-latest')).toBe('acme/gizmo');
  });

  it('keeps parameter sizes and :free as their own families; drops other variants', () => {
    expect(familyKey('acme/zeta3.8-27b:free')).toBe('acme/zeta-27b:free');
    expect(familyKey('acme/zeta3.8-27b')).toBe('acme/zeta-27b');
    expect(familyKey('acme/zeta3-235b-a22b')).toBe('acme/zeta-235b-a22b');
    expect(familyKey('acme/zeta-3.1-8b')).not.toBe(familyKey('acme/zeta-3.1-70b'));
    expect(familyKey('acme/zeta-2:nitro')).toBe('acme/zeta');
  });

  it('mixed letter+digit tokens keep only their letters; an org-less slug still keys', () => {
    expect(familyKey('acme/k2.6-thinking')).toBe('acme/k-thinking');
    expect(familyKey('widget-pro-2')).toBe('widget-pro');
  });

  it('is idempotent on a family key', () => {
    expect(familyKey('deepseek/deepseek-flash')).toBe('deepseek/deepseek-flash');
    expect(familyKey('anthropic/claude-sonnet')).toBe('anthropic/claude-sonnet');
  });
});

describe('familyKey over the bootstrap catalog (contract seam)', () => {
  it('no catalog family key keeps a version or trips the versioned-id pattern', () => {
    const ids = catalogIds();
    expect(ids.length).toBeGreaterThan(10);
    for (const id of ids) {
      const key = familyKey(id);
      expect(key, id).not.toMatch(/\d+\.\d+|(^|[-/])v\d/);
      expect(VERSIONED_CLOUD_MODEL_ID.test(key), `${id} -> ${key}`).toBe(false);
    }
  });

  it('the versioned-id pattern does see the catalog versions that keying removes', () => {
    expect(catalogIds().filter((id) => VERSIONED_CLOUD_MODEL_ID.test(id)).length).toBeGreaterThan(5);
  });

  it('two catalog members of one line share one key (the sonnet ids)', () => {
    const sonnet = catalogIds().filter((id) => id.startsWith('anthropic/') && id.includes('sonnet'));
    expect(sonnet.length).toBeGreaterThanOrEqual(2);
    expect(new Set(sonnet.map(familyKey)).size).toBe(1);
  });
});
