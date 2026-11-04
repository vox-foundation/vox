// Mermaid oracle: the pinned real parser (mermaid@11.17.2 under a jsdom shim) turns Mermaid text
// into canonical id-free sets. It never sees Rust output as anything but text, so it is
// independent of the importer and exporter under test. Test-only: not imported by the site.
import assert, { deepStrictEqual as deepEqual } from 'node:assert/strict';
import { JSDOM } from 'jsdom';

// No runScripts and no resources: the DOM can neither execute script nor fetch anything.
const { window } = new JSDOM('<!doctype html><html><body></body></html>');
globalThis.window = window;
globalThis.document = window.document;
for (const k of ['Element', 'HTMLElement', 'SVGElement', 'Node', 'DocumentFragment', 'DOMParser', 'XMLSerializer']) {
  if (!(k in globalThis)) globalThis[k] = window[k];
}
if (!globalThis.navigator) {
  Object.defineProperty(globalThis, 'navigator', { value: window.navigator, configurable: true });
}
// Dynamic import: a static one would hoist above the shim and crash on DOMPurify.addHook.
const { default: mermaid } = await import('mermaid');
// Default config otherwise: never raise maxEdges or maxTextSize.
mermaid.initialize({ startOnLoad: false, securityLevel: 'strict' });

/** True when the real Mermaid parser accepts `src`. */
export async function accepts(src) {
  try {
    await mermaid.parse(src);
    return true;
  } catch {
    return false;
  }
}

/** Null when the real Mermaid parser accepts `src`, else its own error message (repair feedback). */
export async function parseError(src) {
  try {
    await mermaid.parse(src);
    return null;
  } catch (e) {
    return String(e?.message ?? e);
  }
}

const NAMED = { quot: '"', lt: '<', gt: '>', amp: '&', apos: "'", nbsp: String.fromCharCode(0xa0) };

/** Undo Mermaid's label serialisation (br tags, entity placeholders, HTML entities), then trim. */
export function decodeLabel(s) {
  let t = s.replace(/<br\s*\/?>/gi, '\n');
  t = t.replaceAll('ﬂ°°', '&#').replaceAll('ﬂ°', '&').replaceAll('¶ß', ';');
  t = t.replace(/&(#x[0-9a-f]+|#[0-9]+|[a-z]+);/gi, (m, g) => {
    if (g[0] === '#') {
      return String.fromCodePoint(g[1].toLowerCase() === 'x' ? parseInt(g.slice(2), 16) : parseInt(g.slice(1), 10));
    }
    if (g in NAMED) return NAMED[g];
    throw new Error(`unknown entity ${m}`);
  });
  return t.trim();
}

const SHAPE = { square: 'rect', round: 'rounded', diamond: 'diamond', cylinder: 'cylinder', circle: 'start', doublecircle: 'end' };
const DIR = { TB: 'down', TD: 'down', LR: 'right', RL: 'left', BT: 'up' };
const json = (v) => JSON.stringify(v);

async function canonFlow(src) {
  const { db } = await mermaid.mermaidAPI.getDiagramFromText(src);
  const parentOf = new Map();
  const titleOf = new Map();
  for (const s of db.getSubGraphs()) {
    titleOf.set(s.id, decodeLabel(s.title));
    for (const n of s.nodes) parentOf.set(n, s.id);
  }
  const path = (id) => {
    const out = [];
    for (let c = id; c; c = parentOf.get(c)) out.unshift(titleOf.get(c));
    return out;
  };
  const key = new Map();
  const nodes = [...db.getVertices().values()]
    .filter((v) => !titleOf.has(v.id))
    .map((v) => {
      const k = [decodeLabel(v.text ?? v.id), SHAPE[v.type] ?? 'rect', path(parentOf.get(v.id))];
      key.set(v.id, k);
      return k;
    });
  const edges = db.getEdges().map((e) => [key.get(e.start), key.get(e.end), decodeLabel(e.text || '')]);
  return {
    kind: 'graph',
    direction: DIR[db.getDirection()],
    nodes,
    edges,
    containers: [...titleOf.keys()].map(path),
  };
}

async function canonState(src) {
  const { db } = await mermaid.mermaidAPI.getDiagramFromText(src);
  const states = db.getStates();
  const entries = states instanceof Map ? [...states.entries()] : Object.entries(states);
  const key = new Map();
  for (const [id, st] of entries) {
    // Composite states, notes, forks, choices and dividers are outside the supported subset.
    if (st.doc || (st.type && st.type !== 'default') || st.note) {
      throw new Error(`oracle: unsupported state ${json(id)} (${st.type})`);
    }
    key.set(id, id === 'root_start' ? ['Start', 'start', []] : id === 'root_end' ? ['End', 'end', []] : [decodeLabel(st.descriptions?.[0] ?? id), 'rect', []]);
  }
  const at = (id) => key.get(id) ?? [id, 'rect', []];
  return {
    kind: 'graph',
    direction: DIR[db.getDirection?.() ?? 'TB'],
    nodes: [...key.values()],
    edges: db.getRelations().map((r) => [at(r.id1), at(r.id2), decodeLabel(r.relationTitle ?? '')]),
    containers: [],
  };
}

async function canonSeq(src) {
  const { db } = await mermaid.mermaidAPI.getDiagramFromText(src);
  const names = [...db.getActors().keys()];
  const labels = [...db.getActors().values()].map((a) => decodeLabel(a.description ?? a.name));
  const index = (name) => names.indexOf(name);
  const events = db.getMessages().map((m) => {
    if (m.type === 0 || m.type === 1) return ['message', index(m.from), index(m.to), m.type === 1, decodeLabel(m.message ?? '')];
    if (m.type === 2 && m.placement === 2) {
      const who = m.from === m.to ? [index(m.from)] : [index(m.from), index(m.to)];
      return ['note', who, decodeLabel(m.message ?? '')];
    }
    throw new Error(`oracle: unsupported sequence event type ${m.type} placement ${m.placement}`);
  });
  return { kind: 'sequence', actors: labels, events };
}

/** Canonical sets of a Mermaid block, dispatched on its first non-blank line. */
export async function canon(src) {
  const head = src.split('\n').find((l) => l.trim() !== '')?.trim() ?? '';
  if (/^(graph|flowchart)\b/.test(head)) return canonFlow(src);
  if (/^stateDiagram-v2\b/.test(head)) return canonState(src);
  if (/^sequenceDiagram\b/.test(head)) return canonSeq(src);
  throw new Error(`oracle: unsupported diagram header ${json(head)}`);
}

const sortedJson = (xs) => xs.map(json).sort();

/** Assert two canonical set documents are equal (graph lists as multisets, sequence in order). */
export function sameSets(a, b, message) {
  assert(a.kind === b.kind, `${message}: kind ${a.kind} != ${b.kind}`);
  if (a.kind === 'sequence') {
    deepEqual(a.actors, b.actors, `${message}: actors`);
    deepEqual(a.events, b.events, `${message}: events`);
    return;
  }
  deepEqual(a.direction, b.direction, `${message}: direction`);
  for (const list of ['nodes', 'edges', 'containers']) {
    deepEqual(sortedJson(a[list]), sortedJson(b[list]), `${message}: ${list}`);
  }
}
