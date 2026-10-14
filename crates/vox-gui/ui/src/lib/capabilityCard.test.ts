import { describe, it, expect } from 'vitest';
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { buildExplainCard, SURFACE_CRATES } from './capabilityCard';
import { CAPABILITY_FIXTURE, row } from '../test-fixtures/capabilityIndexFixture';

const DOMAINS = [
  { id: 'compiler', label: 'Compiler' },
  { id: 'mesh', label: 'Mesh / Populi' },
];
const byId = (id: string) => CAPABILITY_FIXTURE.find((r) => r.id === id)!;
const card = (id: string, paths: string[] = ['check']) =>
  buildExplainCard(byId(id), CAPABILITY_FIXTURE, DOMAINS, new Set(paths));

describe('buildExplainCard', () => {
  it('mcp tool card gathers the linked CLI row and offers form + copy actions', () => {
    const c = card('mcp.vox_check');
    expect(c.equivalents.cli).toEqual(['vox check']);
    expect(c.equivalents.mcp).toEqual(['vox_check']);
    expect(c.equivalents.catalog).toBe('generic form');
    expect(c.actions.openForm).toEqual({ path: ['check'] });
    expect(c.actions.copyMcp).toBe('vox_check');
  });

  it('cli card shows the equivalent MCP name and its own invocation', () => {
    const c = card('cli.check');
    expect(c.equivalents.cli).toEqual(['vox check']);
    expect(c.equivalents.mcp).toEqual(['vox_check']);
    expect(c.actions.copyMcp).toBe('vox_check');
    expect(c.status.tier).toBe('recommended');
    expect(c.status.domain).toBe('Compiler · lane —');
  });

  it('side effects: destructive is shown, a null class reads unknown', () => {
    expect(card('cli.plugin.remove', []).status.sideEffects).toBe('destructive');
    expect(card('cli.check').status.sideEffects).toBe('unknown');
  });

  it('truncates MCP names beyond 5 with +N more', () => {
    const tools = Array.from({ length: 7 }, (_, i) =>
      row({ id: `mcp.t${i}`, kind: 'mcp_tool', title: `t${i}`, mcp: `t${i}` }),
    );
    const surface = row({ id: 'surface.x', kind: 'surface', title: 'X', refs: tools.map((t) => t.id) });
    const c = buildExplainCard(surface, [surface, ...tools], DOMAINS, new Set());
    expect(c.equivalents.mcp).toEqual(['t0', 't1', 't2', 't3', 't4', '+2 more']);
  });

  it('related excludes self and equivalents, orders same domain first, caps at 5', () => {
    const c = card('cli.check');
    const ids = c.related.map((r) => r.id);
    expect(ids).not.toContain('cli.check');
    expect(ids).not.toContain('mcp.vox_check');
    expect(ids.length).toBe(5);
    expect(ids).toEqual([...ids].sort());
    expect(ids[0]).toBe('cli.explain');
  });

  it('related: same-domain before same-owner; surface-crate ownership never counts', () => {
    const me = row({ id: 'cli.me', kind: 'cli', title: 'Me', domain: 'a', crate: 'vox-foo', cli: 'vox me' });
    const sameDomain = row({ id: 'cli.z', kind: 'cli', title: 'Z', domain: 'a', crate: 'vox-bar' });
    const sameCrate = row({ id: 'cli.b', kind: 'cli', title: 'B', domain: 'b', crate: 'vox-foo' });
    const surfaceOnly = row({ id: 'cli.c', kind: 'cli', title: 'C', domain: 'b', crate: 'vox-cli' });
    const meSurface = row({ id: 'cli.s', kind: 'cli', title: 'S', domain: 's', crate: 'vox-cli' });
    const all = [me, sameDomain, sameCrate, surfaceOnly, meSurface];
    expect(buildExplainCard(me, all, [], new Set()).related.map((r) => r.id)).toEqual(['cli.z', 'cli.b']);
    expect(buildExplainCard(meSurface, all, [], new Set()).related).toEqual([]);
  });

  it('open form is marked unavailable when the live catalog lacks the path', () => {
    const c = card('cli.check', []);
    expect(c.actions.openForm).toEqual({ path: ['check'], unavailable: "not in this build's command catalog" });
  });

  it('open docs only for docs/src paths, reduced to the docs-relative key', () => {
    const inDocs = row({ id: 'crate.a', kind: 'crate', title: 'a', docs: 'docs/src/architecture/x.md' });
    const other = row({ id: 'crate.b', kind: 'crate', title: 'b', docs: 'README.md' });
    expect(buildExplainCard(inDocs, [inDocs], [], new Set()).actions.openDocs).toBe('architecture/x.md');
    expect(buildExplainCard(other, [other], [], new Set()).actions.openDocs).toBeNull();
    expect(buildExplainCard(other, [other], [], new Set()).location.docs).toBe('README.md');
  });

  it('empty groups render as an em dash, not hidden', () => {
    const c = card('crate.vox-actor-runtime');
    expect(c.equivalents).toEqual({ cli: ['—'], mcp: ['—'], gui: '—', catalog: '—' });
    expect(c.location.docs).toBe('—');
    expect(c.location.source).toBe('—');
    expect(c.location.crate).toBe('vox-actor-runtime (L3)');
    expect(c.related).toEqual([]);
    expect(c.actions).toEqual({ openForm: null, copyMcp: null, openDocs: null });
  });

  it('gui text mirrors gui_text: nav label with view key', () => {
    expect(card('surface.approvals').equivalents.gui).toBe('Approvals (view approvals)');
  });

  it('SURFACE_CRATES equals the Rust constant in vox-capability-registry', () => {
    const src = readFileSync(
      join(import.meta.dirname, '..', '..', '..', '..', 'vox-capability-registry', 'src', 'index.rs'),
      'utf8',
    );
    const m = /SURFACE_CRATES:\s*\[&str;\s*\d+\]\s*=\s*\[([^\]]*)\]/.exec(src);
    expect(m).not.toBeNull();
    const rust = [...m![1].matchAll(/"([^"]+)"/g)].map((x) => x[1]);
    expect([...SURFACE_CRATES]).toEqual(rust);
  });
});
