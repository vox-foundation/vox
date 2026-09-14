import { describe, expect, it } from 'vitest';
import {
  formatSessionBudget,
  isAppSlashCommand,
  parseResearchSlashCommand,
  resolveInternalModeSlash,
  slashCommandBase,
} from './slashRouter';

describe('slashRouter', () => {
  it('normalizes slash tokens to base command', () => {
    expect(slashCommandBase('/plan draft')).toBe('/plan');
    expect(slashCommandBase('  /AUDIT  ')).toBe('/audit');
  });

  it('resolves internal mode slashes', () => {
    expect(resolveInternalModeSlash('/verify run')).toBe('verify');
    expect(resolveInternalModeSlash('/act')).toBe('act');
    expect(resolveInternalModeSlash('/spawn')).toBeNull();
    // /plan moved to APP_SLASH_COMMANDS (Task F1) — it dispatches a real
    // vox_plan turn now, not just an internal Loquela mode switch.
    expect(resolveInternalModeSlash('/plan')).toBeNull();
  });

  it('detects app-level slash commands', () => {
    expect(isAppSlashCommand('/memory')).toBe(true);
    expect(isAppSlashCommand('/rollback now')).toBe(true);
    expect(isAppSlashCommand('/plan')).toBe(true);
  });

  it('formats session budget for display', () => {
    expect(formatSessionBudget(1.234, 50)).toBe('session $1.23 / $50.00');
  });

  it('parses research slash commands with typed flags', () => {
    const p1 = parseResearchSlashCommand('/research --deep --waves=3 --domain=codegen --site=docs.rs tokio async');
    expect(p1).toEqual({
      query: 'tokio async',
      isDeep: true,
      waves: 3,
      domainMode: 'codegen',
      siteScope: 'docs.rs',
    });

    const p2 = parseResearchSlashCommand('/deepresearch best wireless noise cancelling headphones');
    expect(p2).toEqual({
      query: 'best wireless noise cancelling headphones',
      isDeep: true,
      waves: 3,
      domainMode: 'general',
      siteScope: undefined,
    });

    const p3 = parseResearchSlashCommand('/plan create architecture');
    expect(p3).toBeNull();
  });

  it('parses /research search with quotes correctly', () => {
    const res = parseResearchSlashCommand('/research search "rust async concurrency" --min-confidence=0.8');
    expect(res).not.toBeNull();
    expect(res?.subcommand).toBe('search');
    expect(res?.query).toBe('rust async concurrency');
    expect(res?.minConfidence).toBe(0.8);
  });

  it('parses /research-search alias directly', () => {
    const res = parseResearchSlashCommand('/research-search mimalloc jemalloc');
    expect(res).not.toBeNull();
    expect(res?.subcommand).toBe('search');
    expect(res?.query).toBe('mimalloc jemalloc');
  });

  it('parses space-separated flags --domain codegen', () => {
    const res = parseResearchSlashCommand('/research --domain codegen tokio runtime');
    expect(res).not.toBeNull();
    expect(res?.domainMode).toBe('codegen');
    expect(res?.query).toBe('tokio runtime');
  });
});
