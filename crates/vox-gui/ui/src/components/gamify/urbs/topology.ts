// crates/vox-gui/ui/src/components/gamify/urbs/topology.ts
import type { ArchLayer, District, DistrictArchetype, TownCrate } from './types';

export interface ModuleTopologyNode {
  layer: ArchLayer;
  fanIn: number;
  fanOut: number;
  centrality: number;
  rawRank: number;
}

/**
 * Computes pure dependency-graph topological rank (Kahn / longest-path with cycle breaking)
 * and maps every module onto `ArchLayer = 0 | 1 | 2 | 3` (`L0` Bedrock -> `L3` Surface)
 * with zero repository-specific crate name hardcoding.
 */
export function computeTopologicalLayers(crates: TownCrate[]): Map<string, ModuleTopologyNode> {
  const result = new Map<string, ModuleTopologyNode>();
  if (crates.length === 0) return result;

  const known = new Set(crates.map((c) => c.name));
  const depsByModule = new Map<string, string[]>();
  const dependentsByModule = new Map<string, string[]>();
  for (const c of crates) {
    depsByModule.set(c.name, []);
    dependentsByModule.set(c.name, []);
  }

  for (const c of crates) {
    const validDeps = Array.from(new Set((c.deps ?? []).filter((d) => known.has(d) && d !== c.name)));
    depsByModule.set(c.name, validDeps);
    for (const dep of validDeps) {
      dependentsByModule.get(dep)?.push(c.name);
    }
  }

  // Compute longest dependency chain depth (`rawRank`) with cycle guard:
  // rank(u) = 0 if u has no known dependencies, else 1 + max(rank(dep) for dep in deps(u))
  const rankMemo = new Map<string, number>();
  const visiting = new Set<string>();

  const computeDepth = (name: string): number => {
    if (rankMemo.has(name)) return rankMemo.get(name)!;
    if (visiting.has(name)) return 0; // Cycle breaker (SCC safe)
    visiting.add(name);
    const deps = depsByModule.get(name) ?? [];
    let maxDepDepth = -1;
    for (const dep of deps) {
      const d = computeDepth(dep);
      if (d > maxDepDepth) maxDepDepth = d;
    }
    visiting.delete(name);
    const depth = maxDepDepth + 1;
    rankMemo.set(name, depth);
    return depth;
  };

  let maxGraphDepth = 0;
  for (const c of crates) {
    const d = computeDepth(c.name);
    if (d > maxGraphDepth) maxGraphDepth = d;
  }

  // Check if graph has any edges at all
  const hasAnyEdges = crates.some((c) => (depsByModule.get(c.name)?.length ?? 0) > 0);

  for (const c of crates) {
    const fanOut = depsByModule.get(c.name)?.length ?? 0;
    const fanIn = dependentsByModule.get(c.name)?.length ?? 0;
    const rawRank = rankMemo.get(c.name) ?? 0;
    const centrality = (fanIn + 1) * (fanOut + 1);

    let layer: ArchLayer;
    if (!hasAnyEdges) {
      layer = fallbackStructuralLayer(c);
    } else if (fanOut === 0 && fanIn > 0) {
      layer = 0; // Pure bedrock provider
    } else if (fanIn === 0 && fanOut > 0) {
      layer = 3; // Pure leaf consumer / surface entrypoint
    } else if (maxGraphDepth <= 1) {
      layer = rawRank === 0 ? 0 : 2;
    } else {
      const normalized = rawRank / maxGraphDepth;
      if (normalized <= 0.25) layer = 0;
      else if (normalized <= 0.6) layer = 1;
      else if (normalized <= 0.88) layer = 2;
      else layer = 3;
    }

    result.set(c.name, {
      layer,
      fanIn,
      fanOut,
      centrality,
      rawRank,
    });
  }

  return result;
}

/**
 * Fallback layer inference when a repository has no dependency edges at all (e.g. flat scripts).
 * Uses universal structural directory / file extension signals — never repository-specific names.
 */
function fallbackStructuralLayer(crate: TownCrate): ArchLayer {
  const n = crate.name.toLowerCase();
  const root = crate.root.toLowerCase();
  const hasFrontendFiles = crate.files.some(
    (f) => f.path.endsWith('.tsx') || f.path.endsWith('.jsx') || f.path.endsWith('.vue') || f.path.endsWith('.svelte'),
  );
  if (
    hasFrontendFiles ||
    root.startsWith('apps/') ||
    root.startsWith('cmd/') ||
    /(^|[-_/])(ui|web|app|cli|frontend|client|desktop|mobile)($|[-_/])/.test(n)
  ) {
    return 3;
  }
  if (/(^|[-_/])(core|types|primitives|common|shared|utils|config|schema|proto)($|[-_/])/.test(n)) {
    return 0;
  }
  if (
    root.startsWith('services/') ||
    /(^|[-_/])(server|api|gateway|router|worker|scheduler|orchestrator|runtime|hub)($|[-_/])/.test(n)
  ) {
    return 2;
  }
  return 1;
}

/**
 * Classifies any module in any codebase into one of 8 architectural wonder archetypes
 * using pure graph topology, file extensions, and universal structural tokens.
 * Zero hardcoded repository-specific crate names.
 */
export function classifyDistrictArchetype(
  crate: TownCrate,
  topo: { layer: ArchLayer; fanIn: number; fanOut: number; centrality: number },
  totalCrates = 10,
): DistrictArchetype {
  const n = crate.name.toLowerCase();
  const root = crate.root.toLowerCase();
  const files = crate.files ?? [];
  const totalLines = files.reduce((s, f) => s + (f.lines ?? 0), 0);

  const hasUiFiles = files.some(
    (f) => f.path.endsWith('.tsx') || f.path.endsWith('.jsx') || f.path.endsWith('.vue') || f.path.endsWith('.svelte') || f.path.endsWith('.css'),
  );
  const hasTestHeavyFiles =
    files.length > 0 &&
    files.filter((f) => /(\.test\.|\.spec\.|_test\.|tests\/|benches\/|eval)/i.test(f.path)).length / files.length >= 0.4;

  // 1. Circus Arena — benchmark / test / eval / QA harness modules
  if (
    hasTestHeavyFiles ||
    /(^|[-_/])(bench|benches|test|tests|eval|e2e|qa|audit|fuzz|sim|sandbox)($|[-_/])/.test(n)
  ) {
    return 'circus_arena';
  }

  // 2. Amphitheatre Surface — UI / frontend / web / desktop / mobile client apps
  if (
    hasUiFiles ||
    /(^|[-_/])(ui|web|gui|frontend|desktop|mobile|dashboard|portal|view|views|studio)($|[-_/])/.test(n)
  ) {
    return 'amphitheatre_surface';
  }

  // 3. Harbor Vault — persistence / DB / storage / cache / index / search / crypto / secrets
  if (
    /(^|[-_/])(db|database|store|storage|sql|sqlite|postgres|redis|cache|vault|crypto|secrets|auth|search|index|vector|s3|fs)($|[-_/])/.test(
      n,
    )
  ) {
    return 'harbor_vault';
  }

  // 4. Library Scholar — ML / AI / embeddings / RAG / docs / knowledge / training / inference
  if (
    /(^|[-_/])(ml|ai|llm|train|training|infer|inference|embed|embeddings|rag|corpus|knowledge|doc|docs|scholar|model|models|tensor|lora|mens)($|[-_/])/.test(
      n,
    )
  ) {
    return 'library_scholar';
  }

  // 5. Watchtower Leaf — CLI / gateway / webhook / bot / daemon / edge entrypoint (L3 leaf)
  if (
    root.startsWith('cmd/') ||
    /(^|[-_/])(cli|bin|cmd|gateway|webhook|bot|daemon|ingress|edge|proxy)($|[-_/])/.test(n) ||
    (topo.layer === 3 && topo.fanIn === 0 && topo.fanOut > 0)
  ) {
    return 'watchtower_leaf';
  }

  // 6. Explicit Acropolis Engine domain keywords — compiler / parser / kernel / core engine
  if (
    /(^|[-_/])(compiler|parser|lexer|ast|ir|typeck|codegen|engine|kernel|core|vm|interp|protocol|domain)($|[-_/])/.test(
      n,
    )
  ) {
    return 'acropolis_engine';
  }

  // 7. Praetorium Hub — orchestration / server / API / workflow / router / bus / services
  const highFanOutThreshold = Math.max(2, Math.ceil(totalCrates * 0.2));
  if (
    root.startsWith('services/') ||
    /(^|[-_/])(orchestrator|router|server|api|service|workflow|scheduler|runtime|bus|mcp|dispatcher|coordinator|hub|agent|agents)($|[-_/])/.test(
      n,
    ) ||
    (topo.layer === 2 && (topo.fanOut >= highFanOutThreshold || topo.fanIn >= 2))
  ) {
    return 'praetorium_hub';
  }

  // 8. Topological Acropolis fallback for high-fanIn L0-L1 pillars
  const highFanInThreshold = Math.max(2, Math.ceil(totalCrates * 0.25));
  if (topo.layer <= 1 && topo.fanIn >= highFanInThreshold && totalLines >= 300) {
    return 'acropolis_engine';
  }

  // 8. Default: Insula Forum — general module / library district
  return 'insula_forum';
}

const ARCHETYPE_CIVIC_PREFIX: Record<DistrictArchetype, string> = {
  acropolis_engine: 'ACROPOLIS',
  praetorium_hub: 'PRAETORIVM',
  harbor_vault: 'PORTVS',
  amphitheatre_surface: 'AMPHITHEATRVM',
  library_scholar: 'BIBLIOTHECA',
  circus_arena: 'CIRCVS',
  watchtower_leaf: 'SPECVLA',
  insula_forum: 'FORVM',
};

/**
 * Generates a dynamic Roman civic title for any module name + archetype in any language/repo.
 * Strips generic workspace prefixes (`@scope/`, `pkg-`, etc.) and pairs the module stem
 * with its structural archetype prefix.
 */
export function deriveCivicTitle(name: string, archetype: DistrictArchetype): string {
  const clean = name
    .replace(/^@[^/]+\//, '')
    .replace(/^[a-z0-9]+[-_]/i, (prefix) => (name.includes('-') || name.includes('_') ? '' : prefix))
    .replace(/[-_./]+/g, ' ')
    .trim()
    .toUpperCase();
  const stem = clean.length > 0 ? clean : name.toUpperCase();
  const prefix = ARCHETYPE_CIVIC_PREFIX[archetype] ?? 'FORVM';
  return `${prefix} · ${stem}`;
}

export interface BlastRadiusReport {
  /** Direct and transitive modules that `targetModule` depends on (upstream providers). */
  providers: string[];
  /** Direct and transitive modules that depend on `targetModule` (downstream blast radius). */
  impactedDependents: string[];
  /** Subset of `impactedDependents` that are test/eval/surface modules or have test files. */
  staleTestModules: number;
}

/**
 * Computes both the upstream provider chain and the transitive downstream blast radius
 * when `targetModule` is selected or modified.
 */
export function computeBlastRadius(targetModule: string, districts: District[]): BlastRadiusReport {
  const byName = new Map<string, District>();
  const reverseDeps = new Map<string, string[]>();

  for (const d of districts) {
    byName.set(d.name, d);
    reverseDeps.set(d.name, []);
  }
  for (const d of districts) {
    for (const dep of d.deps ?? []) {
      if (byName.has(dep) && dep !== d.name) {
        reverseDeps.get(dep)?.push(d.name);
      }
    }
  }

  // BFS upstream providers (what targetModule depends on)
  const providers: string[] = [];
  const seenProviders = new Set<string>([targetModule]);
  const pQueue: string[] = [...(byName.get(targetModule)?.deps ?? [])];
  while (pQueue.length > 0) {
    const curr = pQueue.shift()!;
    if (!byName.has(curr) || seenProviders.has(curr)) continue;
    seenProviders.add(curr);
    providers.push(curr);
    for (const nextDep of byName.get(curr)?.deps ?? []) {
      if (!seenProviders.has(nextDep)) pQueue.push(nextDep);
    }
  }

  // BFS downstream dependents (what breaks/rebuilds if targetModule changes)
  const impactedDependents: string[] = [];
  const seenImpact = new Set<string>([targetModule]);
  const iQueue: string[] = [...(reverseDeps.get(targetModule) ?? [])];
  while (iQueue.length > 0) {
    const curr = iQueue.shift()!;
    if (!byName.has(curr) || seenImpact.has(curr)) continue;
    seenImpact.add(curr);
    impactedDependents.push(curr);
    for (const nextDependent of reverseDeps.get(curr) ?? []) {
      if (!seenImpact.has(nextDependent)) iQueue.push(nextDependent);
    }
  }

  let staleTestModules = 0;
  for (const modName of impactedDependents) {
    const d = byName.get(modName);
    if (!d) continue;
    const hasTests =
      d.archetype === 'circus_arena' ||
      d.buildings.some((b) => /(\.test\.|\.spec\.|_test\.|tests\/)/i.test(b.path));
    if (hasTests || (d.deps ?? []).includes(targetModule)) {
      staleTestModules++;
    }
  }

  return {
    providers,
    impactedDependents,
    staleTestModules,
  };
}
