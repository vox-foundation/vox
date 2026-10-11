import { describe, it, expect, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import type { CommandCatalogEntry } from '../types/catalog';
import {
  CONFIRMATION_REQUIRED_PREFIX,
  confirmationRequiredId,
  decideCliActivation,
  isReadOnlyClass,
  toastForRun,
} from './capabilityRun';

function entry(over: Partial<CommandCatalogEntry> = {}): CommandCatalogEntry {
  return {
    path: ['features'],
    command: 'features',
    about: 'x',
    aliases: [],
    has_subcommands: false,
    compiled_in: true,
    source_group: 'g',
    feature_gate: null,
    tier: 'recommended',
    arguments: [],
    ...over,
  };
}
const arg = (required: boolean) => ({
  name: 'name',
  short: null,
  long: null,
  help: null,
  required,
  takes_value: true,
  value_kind: 'value' as const,
});

describe('isReadOnlyClass (D-18)', () => {
  it('treats read_only and none as read-only', () => {
    expect(isReadOnlyClass('read_only')).toBe(true);
    expect(isReadOnlyClass('none')).toBe(true);
  });
  it('treats null and every writing class as NOT read-only', () => {
    for (const c of [null, 'writes_files', 'spawns_process', 'destructive', 'network_write', 'bogus']) {
      expect(isReadOnlyClass(c)).toBe(false);
    }
  });
});

describe('decideCliActivation (D-01 safety: Enter never runs a side-effecting verb)', () => {
  it('runs a read_only compiled-in leaf with no required args', () => {
    expect(decideCliActivation({ sideEffect: 'read_only' }, entry())).toEqual({ kind: 'run' });
  });
  it('runs a none-class leaf', () => {
    expect(decideCliActivation({ sideEffect: 'none' }, entry())).toEqual({ kind: 'run' });
  });
  it('runs when only optional args exist', () => {
    expect(decideCliActivation({ sideEffect: 'read_only' }, entry({ arguments: [arg(false)] }))).toEqual({ kind: 'run' });
  });
  it('opens the form for an unclassified (null) verb', () => {
    expect(decideCliActivation({ sideEffect: null }, entry())).toEqual({ kind: 'form' });
  });
  it.each(['writes_files', 'spawns_process', 'destructive'])('opens the form for %s', (c) => {
    expect(decideCliActivation({ sideEffect: c }, entry())).toEqual({ kind: 'form' });
  });
  it('opens the form when a required argument exists', () => {
    expect(decideCliActivation({ sideEffect: 'read_only' }, entry({ arguments: [arg(true)] }))).toEqual({ kind: 'form' });
  });
  it('opens the form for a group node (has subcommands), never runs it', () => {
    expect(decideCliActivation({ sideEffect: 'none' }, entry({ has_subcommands: true }))).toEqual({ kind: 'form' });
  });
  it('explains a verb absent from the live catalog', () => {
    expect(decideCliActivation({ sideEffect: 'read_only' }, undefined)).toEqual({
      kind: 'explain',
      reason: "not in this build's command catalog",
    });
  });
  it('explains a verb that is not compiled in', () => {
    expect(decideCliActivation({ sideEffect: 'read_only' }, entry({ compiled_in: false }))).toEqual({
      kind: 'explain',
      reason: 'not compiled into this build',
    });
  });
});

describe('confirmationRequiredId (D-03 marker)', () => {
  it('parses a string, an Error and a { message } object', () => {
    const m = 'confirmation_required: cli.plugin.remove';
    expect(confirmationRequiredId(m)).toBe('cli.plugin.remove');
    expect(confirmationRequiredId(new Error(m))).toBe('cli.plugin.remove');
    expect(confirmationRequiredId({ message: m })).toBe('cli.plugin.remove');
  });
  it('returns null for anything else', () => {
    expect(confirmationRequiredId('boom')).toBeNull();
    expect(confirmationRequiredId(new Error('other'))).toBeNull();
    expect(confirmationRequiredId(null)).toBeNull();
    expect(confirmationRequiredId(42)).toBeNull();
    expect(confirmationRequiredId('confirmation_required: ')).toBeNull();
  });
});

describe('CONFIRMATION_REQUIRED_PREFIX parity with Rust', () => {
  it('equals the literal in crates/vox-cli/src/command_catalog.rs', () => {
    const src = readFileSync(resolve(import.meta.dirname, '../../../../vox-cli/src/command_catalog.rs'), 'utf8');
    const m = /CONFIRMATION_REQUIRED_PREFIX:\s*&str\s*=\s*"([^"]*)"/.exec(src);
    expect(m).not.toBeNull();
    expect(CONFIRMATION_REQUIRED_PREFIX).toBe(m![1]);
  });
});

describe('toastForRun (D-04)', () => {
  it('ok toast with excerpt and a View full output action', () => {
    const onView = vi.fn();
    const t = toastForRun(['features'], { exit_code: 0, stdout: 'a', stderr: '' }, onView);
    expect(t).toMatchObject({ tone: 'ok', cause: 'backend-ok', title: 'vox features — exit 0', body: 'a' });
    expect(t.action?.label).toBe('View full output');
    t.action?.onClick();
    expect(onView).toHaveBeenCalledTimes(1);
  });
  it('non-zero exit is a warn / backend-error', () => {
    const t = toastForRun(['check'], { exit_code: 2, stdout: '', stderr: 'bad' }, () => {});
    expect(t).toMatchObject({ tone: 'warn', cause: 'backend-error', title: 'vox check — exit 2', body: 'bad' });
  });
  it('bounds the excerpt to 400 characters', () => {
    const t = toastForRun(['x'], { exit_code: 0, stdout: 'z'.repeat(900), stderr: '' }, () => {});
    expect(t.body).toHaveLength(400);
  });
  it('says "no output" when both streams are empty', () => {
    expect(toastForRun(['x'], { exit_code: 0, stdout: '', stderr: '' }, () => {}).body).toBe('no output');
  });
});
