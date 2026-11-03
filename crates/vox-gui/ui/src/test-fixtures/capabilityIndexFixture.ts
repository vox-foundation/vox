/**
 * Shared capability-index fixture for unit tests. Mirrors real ids and values from
 * contracts/capabilities/capability-index.v1.json. Unit tests must use this (and mock
 * `useCapabilityIndex`) instead of importing the 784 KB generated module.
 */
import type { CapabilityGroup, CapabilityIndexEntry } from '../generated/capabilityIndex.generated';

export function row(partial: Partial<CapabilityIndexEntry> & Pick<CapabilityIndexEntry, 'id' | 'kind' | 'title'>): CapabilityIndexEntry {
  return {
    summary: null,
    domain: null,
    cli: null,
    mcp: null,
    viewKey: null,
    crate: null,
    layer: null,
    docs: null,
    docOrigin: null,
    coverage: null,
    coverageReason: null,
    coverageAmbiguous: false,
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

export function group(
  partial: Partial<CapabilityGroup> & Pick<CapabilityGroup, 'name' | 'status' | 'domain'>,
): CapabilityGroup {
  return {
    viewKey: null,
    crate: null,
    layer: null,
    verbs: 1,
    compiled: 1,
    summary: null,
    docs: null,
    rowId: null,
    exemption: null,
    ...partial,
  };
}

export const DOMAIN_FIXTURE: { id: string; label: string }[] = [
  { id: 'compiler', label: 'Compiler' },
  { id: 'orchestrator', label: 'Orchestrator & agents' },
  { id: 'mesh', label: 'Mesh / Populi' },
  { id: 'plugins', label: 'Plugins & skills' },
  { id: 'empty_domain', label: 'Empty Domain' },
];

export const GROUP_FIXTURE: CapabilityGroup[] = [
  group({
    name: 'mesh',
    status: 'dedicated',
    viewKey: 'mesh',
    domain: 'mesh',
    crate: 'vox-populi',
    layer: 2,
    verbs: 2,
    compiled: 1,
    summary: 'Distributed mesh control',
    docs: 'docs/src/architecture/where-things-live.md',
    rowId: 'surface.mesh',
  }),
  group({
    name: 'fmt',
    status: 'generic_form',
    domain: 'compiler',
    crate: 'vox-compiler',
    layer: 1,
    verbs: 1,
    compiled: 1,
    summary: 'CLI operation `vox fmt`',
    docs: 'docs/src/architecture/where-things-live.md',
    rowId: 'cli.fmt',
  }),
  group({
    name: 'attention',
    status: 'cli_only',
    domain: 'orchestrator',
    crate: 'vox-orchestrator',
    layer: 3,
    verbs: 0,
    compiled: 0,
    summary: 'Attention inbox management',
    exemption: {
      owner: 'vox-orchestrator',
      reason: 'Feature-gated behind dei; scheduled for Inbox consolidation.',
      reviewBy: '2026-12-31',
    },
  }),
];

export const CAPABILITY_FIXTURE: CapabilityIndexEntry[] = [
  row({ id: 'cli.check', kind: 'cli', title: 'Check', summary: 'CLI operation `vox check`', domain: 'compiler', cli: 'vox check', tier: 'recommended', equivalents: ['mcp.vox_check'], catalogForm: true }),
  row({ id: 'cli.features', kind: 'cli', title: 'Features', summary: 'CLI operation `vox features`', domain: 'compiler', cli: 'vox features', sideEffect: 'read_only', equivalents: ['mcp.vox_capability_features'], catalogForm: true }),
  row({ id: 'cli.explain', kind: 'cli', title: 'Explain', summary: 'CLI operation `vox explain`', domain: 'compiler', cli: 'vox explain', sideEffect: 'read_only', equivalents: ['mcp.vox_capability_explain'], catalogForm: true }),
  row({ id: 'cli.fmt', kind: 'cli', title: 'Fmt', summary: 'CLI operation `vox fmt`', domain: 'compiler', cli: 'vox fmt', catalogForm: true, docs: 'docs/src/architecture/where-things-live.md' }),
  row({ id: 'cli.fabrica.fmt', kind: 'cli', title: 'Fabrica Fmt', summary: 'CLI operation `vox fabrica fmt`', domain: 'compiler', cli: 'vox fabrica fmt', tier: 'advanced', catalogForm: true }),
  row({ id: 'cli.plugin.remove', kind: 'cli', title: 'Plugin Remove', summary: 'CLI operation `vox plugin remove`', domain: 'plugins', cli: 'vox plugin remove', sideEffect: 'destructive', catalogForm: true }),
  row({ id: 'cli.mesh.status', kind: 'cli', title: 'Mesh Status', summary: 'CLI operation `vox mesh status`', domain: 'mesh', cli: 'vox mesh status', sideEffect: 'read_only', catalogForm: true, docs: 'docs/src/architecture/where-things-live.md' }),
  row({ id: 'cli.mesh.gpu', kind: 'cli', title: 'Mesh GPU', summary: 'CLI operation `vox mesh gpu`', domain: 'mesh', cli: 'vox mesh gpu', catalogForm: false }),
  row({ id: 'mcp.vox_check', kind: 'mcp_tool', title: 'vox_check', domain: 'compiler', mcp: 'vox_check', equivalents: ['cli.check'] }),
  row({ id: 'mcp.vox_capability_explain', kind: 'mcp_tool', title: 'vox_capability_explain', domain: 'compiler', mcp: 'vox_capability_explain', equivalents: ['cli.explain'] }),
  row({ id: 'mcp.vox_capability_features', kind: 'mcp_tool', title: 'vox_capability_features', domain: 'compiler', mcp: 'vox_capability_features', equivalents: ['cli.features'] }),
  row({ id: 'mcp.vox_mesh_nodes', kind: 'mcp_tool', title: 'vox_mesh_nodes', summary: 'List mesh nodes', domain: 'mesh', mcp: 'vox_mesh_nodes', equivalents: [], docs: 'docs/src/architecture/where-things-live.md' }),
  row({ id: 'mcp.vox_orphan_probe', kind: 'mcp_tool', title: 'vox_orphan_probe', summary: 'Orphan probe tool', domain: 'mesh', mcp: 'vox_orphan_probe', equivalents: ['cli.mesh.status'], coverage: 'orphan_backend' }),
  row({ id: 'surface.approvals', kind: 'surface', title: 'Approvals', domain: 'mesh', viewKey: 'approvals', navLabel: 'Approvals' }),
  row({ id: 'surface.mesh', kind: 'surface', title: 'Mesh', domain: 'mesh', viewKey: 'mesh', navLabel: 'Mesh' }),
  row({ id: 'crate.vox-actor-runtime', kind: 'crate', title: 'vox-actor-runtime', crate: 'vox-actor-runtime', layer: 3 }),
  row({ id: 'crate.vox-orchestrator', kind: 'crate', title: 'vox-orchestrator', crate: 'vox-orchestrator', layer: 3 }),
  row({ id: 'crate.vox-populi', kind: 'crate', title: 'vox-populi', crate: 'vox-populi', layer: 2 }),
  row({ id: 'crate.vox-compiler', kind: 'crate', title: 'vox-compiler', crate: 'vox-compiler', layer: 1 }),
  row({ id: 'crate.vox-ast', kind: 'crate', title: 'vox-ast', crate: 'vox-ast', layer: 0 }),
  row({ id: 'skill.algorithmic-art', kind: 'skill', title: 'algorithmic-art', summary: 'Generative art with p5.js' }),
  row({ id: 'doc.docs/src/architecture/where-things-live.md', kind: 'doc', title: 'Where Things Live', docs: 'docs/src/architecture/where-things-live.md' }),
];
