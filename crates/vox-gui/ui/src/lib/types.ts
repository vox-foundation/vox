/**
 * Chat and research types for Vox GUI.
 */

/** Status of an autonomous research run. */
export type ResearchStatus =
  | 'in_progress'
  | 'complete'
  | 'refuted'
  | 'Running'
  | 'Complete'
  | 'Refuted'
  | string;

/** Breakdown of claim verdicts from research verification. */
export interface ResearchClaimsBreakdown {
  supported: number;
  contested: number;
  refuted: number;
}

/** Summary of autonomous research attached to a chat message or response. */
export interface ResearchSummary {
  sessionId?: string;
  query?: string;
  topic?: string;
  status: ResearchStatus;
  claims?: ResearchClaimsBreakdown;
  summary?: string;
  sourcesCount?: number;
  evidenceCount?: number;
  corroborationCount?: number;
  keyFindings?: string[];
}

/** Metadata for chat messages carrying research information. */
export interface ChatMessageResearchMetadata {
  research?: ResearchSummary;
  researchSessionId?: string;
  research_session_id?: string;
}
