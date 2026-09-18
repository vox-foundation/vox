import { invoke } from '@tauri-apps/api/core';

/** The daemon's fire-and-forget research.run envelope. */
export interface ResearchRunHandle {
  session_id: number;
  task_id: string;
  status: string;
}

/**
 * A2: start a research run asynchronously via the persistent orchestrator
 * daemon's `research.run` executor (Tauri command `start_research_async`).
 *
 * Returns the daemon's `{session_id, task_id, status: "running"}` envelope
 * immediately — it does NOT await the pipeline. The caller observes terminal
 * status via the Scientia-queue watcher + session-detail polling. This replaces
 * the old inline `execute_command(['research','run'], …)` path, which blocked
 * the UI for the whole pipeline.
 */
export async function startResearchAsync(args: {
  query: string;
  scope?: string;
  maxSources?: number;
  verifyClaims?: boolean;
}): Promise<ResearchRunHandle> {
  return invoke<ResearchRunHandle>('start_research_async', {
    query: args.query,
    scope: args.scope,
    maxSources: args.maxSources,
    verifyClaims: args.verifyClaims,
  });
}

export interface DocDraftPreview {
  slug: string;
  title: string;
  filename: string;
  markdown_content: string;
  is_valid: boolean;
  validation_errors: string[];
}

export interface PublishDocResult {
  file_path: string;
  relative_path: string;
  indexed: boolean;
}

export async function generateResearchDocDraft(sessionId: number): Promise<DocDraftPreview> {
  return invoke<DocDraftPreview>('generate_research_doc_draft', { sessionId });
}

export async function publishResearchDoc(
  sessionId: number,
  slug: string,
  content: string,
): Promise<PublishDocResult> {
  return invoke<PublishDocResult>('publish_research_doc', { sessionId, slug, content });
}

export interface SandboxProbeOutcome {
  passed: boolean;
  stdout: string;
  stderr: string;
}

export async function executeSandboxProbe(
  code: string,
  language: string,
): Promise<SandboxProbeOutcome> {
  return invoke<SandboxProbeOutcome>('execute_sandbox_probe', { code, language });
}

export interface ProviderProbeResult {
  provider: string;
  http_status: number;
  latency_ms: number;
  success: boolean;
  hit_count: number;
  sample_titles: string[];
  error_message?: string | null;
  remediation_tip?: string | null;
}

export async function probeSearchProvider(
  provider: string,
  query: string,
): Promise<ProviderProbeResult> {
  return invoke<ProviderProbeResult>('probe_search_provider', { provider, query });
}

export async function probeAllSearchProviders(
  query: string,
): Promise<ProviderProbeResult[]> {
  return invoke<ProviderProbeResult[]>('probe_all_search_providers', { query });
}


