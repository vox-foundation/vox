/**
 * Shared capability-index fixture for unit tests. Mirrors real ids and values from
 * contracts/capabilities/capability-index.v1.json. Unit tests must use this (and mock
 * `useCapabilityIndex`) instead of importing the 784 KB generated module.
 */
import type { CapabilityIndexEntry } from '../generated/capabilityIndex.generated';

function row(partial: Partial<CapabilityIndexEntry> & Pick<CapabilityIndexEntry, 'id' | 'kind' | 'title'>): CapabilityIndexEntry {
  return {
    summary: null,
    domain: null,
    cli: null,
    mcp: null,
    viewKey: null,
    crate: null,
    layer: null,
    docs: null,
    coverage: null,
    equivalents: [],
    refs: [],
    tier: null,
    status: 'active',
    sideEffect: null,
    lane: null,
    catalogForm: false,
    navLabel: null,
    source: null,
    ...partial,
  };
}

export const CAPABILITY_FIXTURE: CapabilityIndexEntry[] = [
  row({ id: 'cli.check', kind: 'cli', title: 'Check', summary: 'CLI operation `vox check`', domain: 'compiler', cli: 'vox check', tier: 'recommended', equivalents: ['mcp.vox_check'], catalogForm: true }),
  row({ id: 'cli.features', kind: 'cli', title: 'Features', summary: 'CLI operation `vox features`', domain: 'compiler', cli: 'vox features', sideEffect: 'read_only', equivalents: ['mcp.vox_capability_features'], catalogForm: true }),
  row({ id: 'cli.explain', kind: 'cli', title: 'Explain', summary: 'CLI operation `vox explain`', domain: 'compiler', cli: 'vox explain', sideEffect: 'read_only', equivalents: ['mcp.vox_capability_explain'], catalogForm: true }),
  row({ id: 'cli.fmt', kind: 'cli', title: 'Fmt', summary: 'CLI operation `vox fmt`', domain: 'compiler', cli: 'vox fmt', catalogForm: true }),
  row({ id: 'cli.fabrica.fmt', kind: 'cli', title: 'Fabrica Fmt', summary: 'CLI operation `vox fabrica fmt`', domain: 'compiler', cli: 'vox fabrica fmt', tier: 'advanced', catalogForm: true }),
  row({ id: 'cli.plugin.remove', kind: 'cli', title: 'Plugin Remove', summary: 'CLI operation `vox plugin remove`', domain: 'plugins', cli: 'vox plugin remove', sideEffect: 'destructive', catalogForm: true }),
  row({ id: 'mcp.vox_check', kind: 'mcp_tool', title: 'vox_check', domain: 'compiler', mcp: 'vox_check', equivalents: ['cli.check'] }),
  row({ id: 'mcp.vox_capability_explain', kind: 'mcp_tool', title: 'vox_capability_explain', domain: 'compiler', mcp: 'vox_capability_explain', equivalents: ['cli.explain'] }),
  row({ id: 'mcp.vox_capability_features', kind: 'mcp_tool', title: 'vox_capability_features', domain: 'compiler', mcp: 'vox_capability_features', equivalents: ['cli.features'] }),
  row({ id: 'surface.approvals', kind: 'surface', title: 'Approvals', domain: 'mesh', viewKey: 'approvals', navLabel: 'Approvals' }),
  row({ id: 'surface.mesh', kind: 'surface', title: 'Mesh', domain: 'mesh', viewKey: 'mesh', navLabel: 'Mesh' }),
  row({ id: 'crate.vox-actor-runtime', kind: 'crate', title: 'vox-actor-runtime', crate: 'vox-actor-runtime', layer: 3 }),
  row({ id: 'skill.algorithmic-art', kind: 'skill', title: 'algorithmic-art', summary: 'Generative art with p5.js' }),
  row({ id: 'doc.docs/src/architecture/where-things-live.md', kind: 'doc', title: 'Where Things Live', docs: 'docs/src/architecture/where-things-live.md' }),
];
