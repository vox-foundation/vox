import { describe, it, expect } from 'vitest';
import { LEXICON, sidebarParentLabel } from '../lib/lexicon';
import { NAV_LABELS, labelForNavKey, TOP_LEVEL_VIEWS, PARENT_CHILD_MAP } from '../lib/navigation';
import { SURFACE_REGISTRY } from '../generated/surfaceRegistry.generated';
import { buildFederatedIndex } from '../lib/federatedSearchIndex';
import { chatScopeFiles, readSrc, visibleStrings, propertyStrings } from './sourceScan';

/** Latin and internal code names that must never reach English-mode chrome. */
const CODE_NAMES = /\b(?:Loquela|Oratio|Mercatus|Scientia|Axis Inspector|Secretary|Graphify|VoxGraph)\b/i;

/** "Stop using" column of the canonical vocabulary (chat-surface-design-critique-2026-09-28). */
const RETIRED_TERMS =
  /(?:\bIntents\b|\bauto-route\b|Auto · Router|\bCascade\b|\bBudget burn\b|\bOR Spend\b|\bClutch\b|\bEffic\.|\bBal\.|\bgrounding\b)/i;

const VOCAB_FILES = chatScopeFiles();
/** Label data outside the component scope that renders as chrome. */
const LABEL_DATA_FILES = ['lib/driveConsole.ts', 'hooks/useHudTiles.ts'];

describe('vocabulary patterns (self-test, so the guards below cannot be vacuous)', () => {
  it('CODE_NAMES catches every code name and passes plain English', () => {
    for (const s of ['Loquela', 'Oratio', 'Mercatus', 'Scientia', 'Axis Inspector', 'Secretary suggests a task', 'Dismiss secretary toast', 'Graphify Corpus Health', 'VoxGraph']) {
      expect(CODE_NAMES.test(s), s).toBe(true);
    }
    for (const s of ['Market', 'Findings', 'Suggested task', 'Search Index Health', 'Voice', 'Routing Axes', 'Review']) {
      expect(CODE_NAMES.test(s), s).toBe(false);
    }
  });

  it('RETIRED_TERMS catches the stop-using column and passes the canonical terms', () => {
    for (const s of ['Intents', 'auto-route (clear override)', 'Auto · Router', 'Cloud · Cascade', 'Budget burn', 'OR Spend', 'Clutch — how much to spend', 'Effic.', 'Bal.', 'grounding: off', 'Confirm + enforce grounding.']) {
      expect(RETIRED_TERMS.test(s), s).toBe(true);
    }
    for (const s of ['Routing', 'Spend', 'Attention', 'Mode', 'Risk: Moderate', 'Check replies', 'Needs you', 'Efficient', 'Balanced', 'Intent', 'Background task', 'Auto', 'Cloud']) {
      expect(RETIRED_TERMS.test(s), s).toBe(false);
    }
  });

  it('propertyStrings sees object-literal labels in both quote styles', () => {
    expect(propertyStrings(`const T = [{ id: "auto", label: "Auto · Router" }]; const C = { moderate: 'Confirm + enforce grounding.' };`))
      .toEqual(['auto', 'Auto · Router', 'Confirm + enforce grounding.']);
  });
});

describe('one label source: LEXICON', () => {
  it('every English lexicon label is plain English (never its Latin form, never a code name)', () => {
    const offenders = Object.entries(LEXICON)
      .filter(([, e]) => (e.la !== undefined && e.en === e.la) || CODE_NAMES.test(e.en))
      .map(([k, e]) => `${k}: ${e.en}`);
    expect(offenders).toEqual([]);
  });

  it('NAV_LABELS covers every nav key and is derived from LEXICON', () => {
    const navKeys = [...new Set<string>([...TOP_LEVEL_VIEWS, ...Object.keys(PARENT_CHILD_MAP)])].sort();
    expect(Object.keys(NAV_LABELS).sort()).toEqual(navKeys);
    for (const k of navKeys) expect(NAV_LABELS[k], k).toBe(sidebarParentLabel(k, 'en'));
    expect(labelForNavKey('mercatus')).toBe('Market');
    expect(labelForNavKey('runs')).toBe('Review');
    expect(labelForNavKey('vox-search')).toBe('Search Index');
  });

  it('the command palette names each surface with its lexicon label, not the registry navLabel', () => {
    // Two sources that disagree today: the generated registry says "Mercatus", "Agents" (flow),
    // "Commands" (catalog); LEXICON says "Market", "Flow", "Catalog". The lexicon wins.
    const index = buildFederatedIndex({ surfaces: SURFACE_REGISTRY, settings: [], policies: [], docs: [], skills: [] });
    const offenders: string[] = [];
    for (const s of SURFACE_REGISTRY) {
      if (!s.viewKey || !s.navLabel) continue;
      const row = index.find((r) => r.id === `surface:${s.viewKey}`);
      const want = LEXICON[s.viewKey]?.en ?? s.navLabel;
      if (!row || row.label !== want || CODE_NAMES.test(row.label)) offenders.push(`${s.viewKey}: ${row?.label}`);
    }
    expect(offenders).toEqual([]);
  });
});

describe('English-mode chrome in chat, composer and status bar', () => {
  it('scans a real, non-empty file set', () => {
    expect(VOCAB_FILES).toContain('components/layout/BottomStatusBar.tsx');
    expect(VOCAB_FILES).toContain('components/common/StatusBarCluster.tsx');
    expect(VOCAB_FILES).toContain('components/surfaces/Loquela/Loquela.tsx');
    expect(VOCAB_FILES.length).toBeGreaterThanOrEqual(12);
  });

  it('no visible string contains a Latin or code name', () => {
    const offenders = VOCAB_FILES.flatMap((rel) =>
      visibleStrings(readSrc(rel))
        .filter((t) => CODE_NAMES.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect(offenders).toEqual([]);
  });

  it('no visible string or label datum uses a retired term', () => {
    const fromChrome = VOCAB_FILES.flatMap((rel) =>
      [...visibleStrings(readSrc(rel)), ...propertyStrings(readSrc(rel))]
        .filter((t) => RETIRED_TERMS.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    const fromData = LABEL_DATA_FILES.flatMap((rel) =>
      propertyStrings(readSrc(rel))
        .filter((t) => RETIRED_TERMS.test(t))
        .map((t) => `${rel}: ${t}`),
    );
    expect([...fromChrome, ...fromData]).toEqual([]);
  });
});
