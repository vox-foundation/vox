// crates/vox-gui/ui/src/components/gamify/urbs/__tests__/topology.test.ts
import { describe, it, expect } from 'vitest';
import {
  classifyDistrictArchetype,
  computeBlastRadius,
  computeTopologicalLayers,
  deriveCivicTitle,
} from '../topology';
import { layoutTown } from '../layout';
import type { TownCrate, TownScan } from '../types';

describe('topology (Phase 260: Zero-Hardcoding Graph Topology & Structural Archetype Classifier)', () => {
  it('computes L0..L3 topological layers purely from DAG edges without hardcoded crate names', () => {
    const crates: TownCrate[] = [
      {
        name: 'shared-types',
        root: 'packages/shared-types',
        files: [{ path: 'packages/shared-types/index.ts', lines: 120 }],
        deps: [],
      },
      {
        name: 'domain-kernel',
        root: 'packages/domain-kernel',
        files: [{ path: 'packages/domain-kernel/engine.ts', lines: 500 }],
        deps: ['shared-types'],
      },
      {
        name: 'workflow-service',
        root: 'services/workflow-service',
        files: [{ path: 'services/workflow-service/router.ts', lines: 650 }],
        deps: ['domain-kernel', 'shared-types'],
      },
      {
        name: 'customer-portal',
        root: 'apps/customer-portal',
        files: [{ path: 'apps/customer-portal/App.tsx', lines: 420 }],
        deps: ['workflow-service'],
      },
    ];

    const topo = computeTopologicalLayers(crates);
    expect(topo.get('shared-types')?.layer).toBe(0);
    expect(topo.get('shared-types')?.fanIn).toBe(2);
    expect(topo.get('shared-types')?.fanOut).toBe(0);

    expect(topo.get('domain-kernel')?.layer).toBe(1);
    expect(topo.get('workflow-service')?.layer).toBe(2);
    expect(topo.get('customer-portal')?.layer).toBe(3);
    expect(topo.get('customer-portal')?.fanIn).toBe(0);
  });

  it('handles cyclic dependency graphs (SCCs) gracefully without infinite loops', () => {
    const cyclicCrates: TownCrate[] = [
      {
        name: 'alpha-mod',
        root: 'packages/alpha-mod',
        files: [{ path: 'packages/alpha-mod/a.ts', lines: 100 }],
        deps: ['beta-mod'],
      },
      {
        name: 'beta-mod',
        root: 'packages/beta-mod',
        files: [{ path: 'packages/beta-mod/b.ts', lines: 100 }],
        deps: ['alpha-mod'],
      },
    ];

    const topo = computeTopologicalLayers(cyclicCrates);
    expect(topo.has('alpha-mod')).toBe(true);
    expect(topo.has('beta-mod')).toBe(true);
  });

  it('classifies diverse polyglot modules into structural wonder archetypes and derives dynamic civic titles', () => {
    const crates: TownCrate[] = [
      {
        name: 'query-parser',
        root: 'packages/query-parser',
        files: [{ path: 'packages/query-parser/lexer.ts', lines: 800 }],
        deps: [],
      },
      {
        name: 'job-scheduler',
        root: 'services/job-scheduler',
        files: [{ path: 'services/job-scheduler/worker.ts', lines: 600 }],
        deps: ['query-parser'],
      },
      {
        name: 'postgres-store',
        root: 'packages/postgres-store',
        files: [{ path: 'packages/postgres-store/pool.ts', lines: 400 }],
        deps: [],
      },
      {
        name: 'web-dashboard',
        root: 'apps/web-dashboard',
        files: [{ path: 'apps/web-dashboard/View.tsx', lines: 500 }],
        deps: ['job-scheduler'],
      },
      {
        name: 'rag-embeddings',
        root: 'packages/rag-embeddings',
        files: [{ path: 'packages/rag-embeddings/embed.py', lines: 350 }],
        deps: ['postgres-store'],
      },
      {
        name: 'e2e-bench',
        root: 'packages/e2e-bench',
        files: [{ path: 'packages/e2e-bench/load.spec.ts', lines: 250 }],
        deps: ['web-dashboard'],
      },
      {
        name: 'admin-cli',
        root: 'cmd/admin-cli',
        files: [{ path: 'cmd/admin-cli/main.go', lines: 180 }],
        deps: ['job-scheduler'],
      },
    ];

    const topo = computeTopologicalLayers(crates);
    const byName = new Map(crates.map((c) => [c.name, c]));

    const parserArch = classifyDistrictArchetype(
      byName.get('query-parser')!,
      topo.get('query-parser')!,
      crates.length,
    );
    expect(parserArch).toBe('acropolis_engine');
    expect(deriveCivicTitle('query-parser', parserArch)).toContain('ACROPOLIS');

    const schedArch = classifyDistrictArchetype(
      byName.get('job-scheduler')!,
      topo.get('job-scheduler')!,
      crates.length,
    );
    expect(schedArch).toBe('praetorium_hub');
    expect(deriveCivicTitle('job-scheduler', schedArch)).toContain('PRAETORIVM');

    const storeArch = classifyDistrictArchetype(
      byName.get('postgres-store')!,
      topo.get('postgres-store')!,
      crates.length,
    );
    expect(storeArch).toBe('harbor_vault');

    const dashArch = classifyDistrictArchetype(
      byName.get('web-dashboard')!,
      topo.get('web-dashboard')!,
      crates.length,
    );
    expect(dashArch).toBe('amphitheatre_surface');

    const ragArch = classifyDistrictArchetype(
      byName.get('rag-embeddings')!,
      topo.get('rag-embeddings')!,
      crates.length,
    );
    expect(ragArch).toBe('library_scholar');

    const benchArch = classifyDistrictArchetype(
      byName.get('e2e-bench')!,
      topo.get('e2e-bench')!,
      crates.length,
    );
    expect(benchArch).toBe('circus_arena');

    const cliArch = classifyDistrictArchetype(
      byName.get('admin-cli')!,
      topo.get('admin-cli')!,
      crates.length,
    );
    expect(cliArch).toBe('watchtower_leaf');
  });

  it('computes upstream providers and transitive downstream blast radius accurately', () => {
    const scan: TownScan = {
      root: '/repo',
      scanned_at_ms: 0,
      truncated: false,
      crates: [
        {
          name: 'core-schema',
          root: 'packages/core-schema',
          files: [{ path: 'packages/core-schema/schema.ts', lines: 100 }],
          deps: [],
        },
        {
          name: 'auth-engine',
          root: 'packages/auth-engine',
          files: [{ path: 'packages/auth-engine/jwt.ts', lines: 200 }],
          deps: ['core-schema'],
        },
        {
          name: 'api-gateway',
          root: 'services/api-gateway',
          files: [{ path: 'services/api-gateway/server.ts', lines: 300 }],
          deps: ['auth-engine'],
        },
        {
          name: 'api-e2e-tests',
          root: 'packages/api-e2e-tests',
          files: [{ path: 'packages/api-e2e-tests/gateway.test.ts', lines: 150 }],
          deps: ['api-gateway'],
        },
      ],
    };

    const layout = layoutTown(scan, new Set());
    const authBlast = computeBlastRadius('auth-engine', layout.districts);
    expect(authBlast.providers).toEqual(['core-schema']);
    expect(authBlast.impactedDependents).toEqual(['api-gateway', 'api-e2e-tests']);
    expect(authBlast.staleTestModules).toBeGreaterThanOrEqual(1);
  });
});
