export type StageStatus = 'done' | 'active' | 'pending' | 'error';

// Mirrors crates/vox-cli/src/commands/research/mod.rs:234-243 (no Rust enum exists).
export const RESEARCH_STAGES = [
  'queued', 'planning', 'retrieving', 'verifying_claims',
  'synthesizing', 'auditing_citations', 'persisting_artifact', 'completed',
] as const;

// Mirrors the scientia_publication_queue lifecycle.
export const PUBLICATION_STAGES = [
  'draft', 'doi_reserved', 'orcid_attributed', 'approved', 'submitted', 'published', 'failed',
] as const;

/**
 * Coarse-grained per-stage status derived from the persisted session status.
 * The DB does not track per-stage progress, so this is intentionally coarse:
 * completed → all done; failed/orphaned → all error; otherwise queued done, rest pending.
 */
export function deriveStages(sessionStatus: string): Record<string, StageStatus> {
  const out: Record<string, StageStatus> = {};
  const status = sessionStatus.toLowerCase();
  const activeIdx = (RESEARCH_STAGES as readonly string[]).indexOf(status);
  for (let i = 0; i < RESEARCH_STAGES.length; i++) {
    const stage = RESEARCH_STAGES[i];
    if (status === 'completed') {
      out[stage] = 'done';
    } else if (status === 'failed' || status === 'orphaned') {
      out[stage] = 'error';
    } else if (activeIdx > 0) {
      if (i < activeIdx) out[stage] = 'done';
      else if (i === activeIdx) out[stage] = 'active';
      else out[stage] = 'pending';
    } else {
      out[stage] = stage === 'queued' ? 'done' : 'pending';
    }
  }
  return out;
}

export interface PublicationManifest {
  publication_id: string;
  content_type: string;
  state: string;
  created_at_ms: number;
  updated_at_ms: number;
  title?: string | null;
  author?: string | null;
  abstract_text?: string | null;
  candidate_class?: string | null;
  publication_tier?: string | null;
  arxiv_primary_category?: string | null;
  arxiv_secondary_categories?: string[] | null;
  license_spdx?: string | null;
  orcid?: string | null;
  ror_affiliation?: string | null;
  prereserved_doi?: string | null;
  arxiv_id?: string | null;
  readiness_score?: number | null;
  arxiv_ready?: boolean | null;
  flywheel_stages_done?: string[] | null;
}

/** Bucket manifests by `state`, preserving every canonical stage (empty allowed). */
export function groupByStage(manifests: PublicationManifest[]): Record<string, PublicationManifest[]> {
  const groups: Record<string, PublicationManifest[]> = {};
  for (const s of PUBLICATION_STAGES) groups[s] = [];
  for (const m of manifests) {
    const key = m.state === 'archived' ? 'published' : m.state;
    (groups[key] ??= []).push(m);
  }
  return groups;
}
