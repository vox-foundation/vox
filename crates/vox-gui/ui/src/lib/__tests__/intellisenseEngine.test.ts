// @vitest-environment jsdom
import { describe, it, expect } from 'vitest';
import {
  computeIntelliSenseItems,
  fuzzyScore,
  parseCompletionTrigger,
} from '../intellisenseEngine';

describe('intellisenseEngine', () => {
  it('detects triggers for vox, /slash, !nu, pipe |, and @mentions', () => {
    expect(parseCompletionTrigger('vox config').kind).toBe('vox');
    expect(parseCompletionTrigger('/mod').kind).toBe('slash');
    expect(parseCompletionTrigger('!nu ls').kind).toBe('nu');
    expect(parseCompletionTrigger('!nu ls | wh').kind).toBe('pipe');
    expect(parseCompletionTrigger('explain @term:failed').kind).toBe('at_term');
    expect(parseCompletionTrigger('check @mcp:vox_').kind).toBe('at_mcp');
    expect(parseCompletionTrigger('use @symbol:comp').kind).toBe('at_symbol');
  });

  it('scores exact and prefix matches higher than subsequence matches', () => {
    const exact = fuzzyScore('doctor', 'doctor');
    const prefix = fuzzyScore('doc', 'doctor');
    const subseq = fuzzyScore('dtr', 'doctor');
    expect(exact).toBeGreaterThan(prefix);
    expect(prefix).toBeGreaterThan(subseq);
    expect(subseq).toBeGreaterThan(0);
  });

  it('merges 7 sources and computes inline ghostSuffix for matching prefixes', () => {
    const voxItems = computeIntelliSenseItems({
      input: 'vox config',
      surface: 'terminal',
      catalogSuggestions: [
        {
          action_id: 'vox.config.show',
          completion: 'config show',
          about: 'Show active configuration',
          score: 900,
        },
      ],
    });
    expect(voxItems.length).toBeGreaterThan(0);
    expect(voxItems[0].label).toBe('vox config show');
    expect(voxItems[0].ghostSuffix).toBe(' show');

    const atItems = computeIntelliSenseItems({
      input: 'check @',
      surface: 'chat',
      skills: [{ name: 'brainstorming', description: 'Explore design' }],
      repoFiles: ['crates/vox-cli/src/lib.rs'],
      agents: [{ id: 'agent-1', codename: 'Argon', phase: 'active' }],
      termBridge: {
        activeSessionId: 's-1',
        blocks: [],
        pinnedBlockIds: [],
      },
    });
    const categories = new Set(atItems.map((i) => i.category));
    expect(categories.has('term')).toBe(true);
    expect(categories.has('file')).toBe(true);
    expect(categories.has('skill')).toBe(true);
    expect(categories.has('mcp')).toBe(true);
    expect(categories.has('symbol')).toBe(true);
  });

  it('boosts failed terminal blocks and recovery suggestions when a block failed', () => {
    const items = computeIntelliSenseItems({
      input: '',
      surface: 'terminal',
      forceOpen: true,
      termBridge: {
        activeSessionId: 's-1',
        blocks: [
          {
            id: 7,
            kind: 'Shell',
            status: 'Failed',
            input: 'cargo test',
            output: [],
            exit_code: 101,
            started_ms: 1,
            ended_ms: 2,
          },
        ],
        pinnedBlockIds: [],
      },
    });
    expect(items.some((i) => i.label === '?fix error in last block')).toBe(true);
    expect(items.some((i) => i.label === '@term:s-1#7')).toBe(true);
  });
});
