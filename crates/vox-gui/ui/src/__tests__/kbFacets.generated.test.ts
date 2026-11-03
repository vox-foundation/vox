import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import {
  AUTHORSHIP_VALUES,
  KB_FACET_KEYS,
  KB_TIER_VALUES,
  ORIGIN_VALUES,
  SCOPE_VALUES,
  SENSITIVITY_VALUES,
  TRUST_TIER_VALUES,
  type KbFacets,
} from '../generated/kbFacets.generated';

// src/__tests__ -> src -> ui -> vox-gui -> crates -> repo root
const schema = JSON.parse(
  readFileSync(
    join(__dirname, '..', '..', '..', '..', '..', 'contracts', 'documentation', 'kb-facets.v1.schema.json'),
    'utf8',
  ),
);

describe('kbFacets.generated.ts vs committed kb-facets.v1.schema.json', () => {
  const enums: Record<string, readonly string[]> = {
    Authorship: AUTHORSHIP_VALUES,
    KbTier: KB_TIER_VALUES,
    Origin: ORIGIN_VALUES,
    Scope: SCOPE_VALUES,
    Sensitivity: SENSITIVITY_VALUES,
    TrustTier: TRUST_TIER_VALUES,
  };

  it.each(Object.entries(enums))('%s values equal the schema enum', (name, values) => {
    expect([...values]).toEqual(schema.definitions[name].enum);
  });

  it('covers every enum definition in the schema', () => {
    const schemaEnums = Object.entries<{ enum?: string[] }>(schema.definitions)
      .filter(([, d]) => Array.isArray(d.enum))
      .map(([n]) => n)
      .sort();
    expect(Object.keys(enums).sort()).toEqual(schemaEnums);
  });

  it('KB_FACET_KEYS equals the schema root properties in order', () => {
    expect([...KB_FACET_KEYS]).toEqual(Object.keys(schema.properties));
  });

  it('a fixture using every enum compiles against KbFacets', () => {
    const fixture = {
      topic: 'search',
      domain: 'platform',
      kb_tier: KB_TIER_VALUES[0],
      trust_tier: TRUST_TIER_VALUES[0],
      scope: SCOPE_VALUES[0],
      authorship: AUTHORSHIP_VALUES[0],
      origin: ORIGIN_VALUES[0],
      sensitivity: SENSITIVITY_VALUES[0],
      training_eligible: true,
      sources: [{ url: 'https://example.com', date: '2026-10-10', hash: null }],
      supersedes: [],
    } satisfies KbFacets;
    expect(Object.keys(fixture).every((k) => (KB_FACET_KEYS as readonly string[]).includes(k))).toBe(true);
  });
});
