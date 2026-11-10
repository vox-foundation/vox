import { describe, expect, it } from 'vitest';
import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import {
  THEME_CONTRACT,
  REQUIRED_SEMANTIC_CSS_VARS,
  auditSourceTextAgainstThemeContract,
  getThemeContractHealthReport,
  verifyThemeProfileContrast,
  computeContrastRatio,
  type ThemeContractDriftFinding,
} from './themeContract';
import { applyTheme, isMinimalistDarkTheme } from './theme';
import { discoverAllGuiSurfacesAndPanels } from './cvSurfaceDiscovery';
import { conceptArtPlateForSurface } from '../../e2e/review/vlmVisionFlywheel';

const __dirname = dirname(fileURLToPath(import.meta.url));
const UI_ROOT = join(__dirname, '..', '..');
const SRC_ROOT = join(UI_ROOT, 'src');
const REPO_ROOT = join(UI_ROOT, '..', '..', '..');
const JSON_CONTRACT_PATH = join(REPO_ROOT, 'contracts', 'gui', 'theme-contract.v1.json');

function walkSourceFiles(dir: string, acc: string[] = []): string[] {
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    const st = statSync(full);
    if (st.isDirectory()) {
      walkSourceFiles(full, acc);
    } else if (
      entry.endsWith('.tsx') ||
      entry.endsWith('.ts') ||
      entry.endsWith('.css')
    ) {
      acc.push(full);
    }
  }
  return acc;
}

describe('Phase 386 — Enforceable & Self-Expanding GUI Theme Contract', () => {
  it('keeps contracts/gui/theme-contract.v1.json in strict parity with runtime THEME_CONTRACT', () => {
    const rawJson = JSON.parse(readFileSync(JSON_CONTRACT_PATH, 'utf8'));
    expect(rawJson.schema_version).toBe(THEME_CONTRACT.schema_version);
    expect(rawJson.contract_id).toBe(THEME_CONTRACT.contract_id);
    expect(rawJson.default_theme).toBe(THEME_CONTRACT.default_theme);
    expect(rawJson.supported_themes).toEqual([...THEME_CONTRACT.supported_themes]);
    expect(rawJson.required_semantic_css_vars).toEqual([...REQUIRED_SEMANTIC_CSS_VARS]);
    expect(rawJson.themes.arcane.tokens).toEqual(THEME_CONTRACT.themes.arcane.tokens);
    expect(rawJson.themes.slate.tokens).toEqual(THEME_CONTRACT.themes.slate.tokens);
    expect(rawJson.themes['high-contrast'].tokens).toEqual(
      THEME_CONTRACT.themes['high-contrast'].tokens,
    );
    expect(rawJson.themes.glacier.tokens).toEqual(THEME_CONTRACT.themes.glacier.tokens);
    expect(rawJson.banned_hardcoded_dark_brown_hexes).toEqual([
      ...THEME_CONTRACT.banned_hardcoded_dark_brown_hexes,
    ]);
    expect(rawJson.banned_romanesque_font_literals).toEqual([
      ...THEME_CONTRACT.banned_romanesque_font_literals,
    ]);
  });

  it('defines all 22 required semantic CSS variables individually in every generated Style Dictionary theme file (arcane, slate, high-contrast, travertine)', () => {
    const indexCss = readFileSync(join(SRC_ROOT, 'index.css'), 'utf8');
    const generatedFiles = [
      { name: 'arcane (tokens.generated.css)', file: 'tokens.generated.css' },
      { name: 'slate (tokens.slate.generated.css)', file: 'tokens.slate.generated.css' },
      { name: 'high-contrast (tokens.contrast.generated.css)', file: 'tokens.contrast.generated.css' },
      { name: 'glacier/travertine (tokens.travertine.generated.css)', file: 'tokens.travertine.generated.css' },
    ];

    for (const { name, file } of generatedFiles) {
      const cssContent = readFileSync(join(SRC_ROOT, 'styles', file), 'utf8');
      for (const cssVar of REQUIRED_SEMANTIC_CSS_VARS) {
        expect(
          cssContent.includes(`${cssVar}:`),
          `Generated stylesheet ${name} is missing semantic variable ${cssVar}`,
        ).toBe(true);
      }
    }

    // Verify single-line button and pill badge invariant in index.css
    expect(indexCss).toContain('white-space: nowrap');
  });

  it('enforces programmatic WCAG 2.1 AA contrast ratios (>= 4.5:1) across all 4 theme profiles', () => {
    const profiles = [
      THEME_CONTRACT.themes.arcane,
      THEME_CONTRACT.themes.slate,
      THEME_CONTRACT.themes['high-contrast'],
      THEME_CONTRACT.themes.glacier,
    ] as const;

    for (const profile of profiles) {
      const checks = verifyThemeProfileContrast(profile);
      expect(checks.length).toBe(9);
      for (const check of checks) {
        expect(
          check.passed,
          `Theme '${profile.id}' failed WCAG contrast on ${check.foregroundVar} (${check.foregroundHex}) over ${check.backgroundVar} (${check.backgroundHex}): ratio ${check.ratio} < ${check.minRequired}`,
        ).toBe(true);
      }
    }

    // Sanity check contrast math on dark espresso over Carrara marble
    expect(computeContrastRatio('#1C140C', '#FAF6EE')).toBeGreaterThan(14);
  });

  it('catches adversarial off-by-one dark hexes, inline dark RGBAs, and unquoted banned fonts', () => {
    const adversarialSample = [
      '<div className="rounded bg-[#251C0E] from-[#19120B] text-white">',
      '  <div style={{ background: "linear-gradient(180deg, rgba(19, 16, 12, 0.97), rgba(11, 9, 6, 0.99))" }} />',
      '  <span style={{ fontFamily: "Courier, monospace" }}>bad</span>',
      '</div>',
    ].join('\n');

    const findings = auditSourceTextAgainstThemeContract(
      'src/components/AdversarialWidget.tsx',
      adversarialSample,
    );
    expect(findings.some((f) => f.kind === 'banned-dark-brown-hex' && f.token === '#251c0e')).toBe(
      true,
    );
    expect(findings.some((f) => f.kind === 'banned-dark-brown-hex' && f.token === '#19120b')).toBe(
      true,
    );
    expect(findings.some((f) => f.kind === 'banned-dark-brown-rgba')).toBe(true);
    expect(findings.some((f) => f.kind === 'banned-font-literal' && f.token === 'Courier')).toBe(
      true,
    );
  });

  it('automatically scans all .tsx, .ts, and .css files in src/ and enforces zero dark-brown hex or banned font drift', () => {
    const files = walkSourceFiles(SRC_ROOT);
    expect(files.length).toBeGreaterThan(150);

    const allFindings: ThemeContractDriftFinding[] = [];
    for (const absPath of files) {
      const rel = relative(UI_ROOT, absPath).replace(/\\/g, '/');
      const content = readFileSync(absPath, 'utf8');
      const findings = auditSourceTextAgainstThemeContract(rel, content);
      allFindings.push(...findings);
    }

    expect(
      allFindings,
      `Theme Contract drift detected across src/ files:\n${allFindings
        .map((f) => `  - ${f.file}:${f.line} [${f.kind}] ${f.token}: ${f.message}`)
        .join('\n')}`,
    ).toEqual([]);
  });

  it('auto-enrolls every registered GUI surface and Chat dock panel into the Theme Contract and CV concept-art mapping', () => {
    const discovery = discoverAllGuiSurfacesAndPanels();
    const arcaneReport = getThemeContractHealthReport('arcane');
    const slateReport = getThemeContractHealthReport('slate');

    expect(arcaneReport.enforced).toBe(true);
    expect(arcaneReport.activeThemeId).toBe('arcane');
    expect(arcaneReport.activeThemeMode).toBe('light');
    expect(arcaneReport.missingSemanticVars).toEqual([]);
    expect(arcaneReport.enrolledSurfaceCount).toBe(discovery.routedViewKeys.length);
    expect(arcaneReport.enrolledChatDockCount).toBe(discovery.chatDockPanelIds.length);

    expect(slateReport.enforced).toBe(true);
    expect(slateReport.activeThemeId).toBe('slate');
    expect(slateReport.activeThemeMode).toBe('dark');
    expect(slateReport.missingSemanticVars).toEqual([]);

    for (const surfaceId of discovery.routedViewKeys) {
      const plate = conceptArtPlateForSurface(surfaceId);
      expect(plate.length).toBeGreaterThan(10);
    }
    for (const dockId of discovery.chatDockPanelIds) {
      const plate = conceptArtPlateForSurface(dockId);
      expect(plate.length).toBeGreaterThan(10);
    }
  });

  it('switches live DOM data-theme attributes cleanly across arcane, slate, and void', () => {
    expect(applyTheme('arcane')).toBe('arcane');
    expect(document.documentElement.dataset.theme).toBe('arcane');
    expect(isMinimalistDarkTheme('arcane')).toBe(false);

    expect(applyTheme('slate')).toBe('slate');
    expect(document.documentElement.dataset.theme).toBe('slate');
    expect(isMinimalistDarkTheme('slate')).toBe(true);

    expect(applyTheme('void')).toBe('void');
    expect(document.documentElement.dataset.theme).toBe('void');
    expect(isMinimalistDarkTheme('void')).toBe(true);

    // Restore default
    applyTheme('arcane');
  });
});
