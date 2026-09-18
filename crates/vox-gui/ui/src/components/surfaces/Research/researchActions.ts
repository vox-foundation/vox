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
  waves?: number;
  domainMode?: string;
  lane?: 'fast' | 'deep';
}): Promise<ResearchRunHandle> {
  return invoke<ResearchRunHandle>('start_research_async', {
    query: args.query,
    scope: args.scope,
    maxSources: args.maxSources,
    verifyClaims: args.verifyClaims,
    waves: args.waves,
    domainMode: args.domainMode,
    lane: args.lane,
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

export interface QuotaUsageDto {
  units_spent: number;
  units_limit: number;
  last_synced_at: string;
}

export interface ProviderStatusDto {
  id: string;
  name: string;
  is_keyless: boolean;
  is_enabled: boolean;
  has_key: boolean;
  quota_usage?: QuotaUsageDto | null;
}

export interface FreeTierOffer {
  provider_id: string;
  provider_name: string;
  signup_url: string;
  monthly_free_units: number;
  headline_benefit: string;
  docs_remediation: string;
}

export interface ResearchEngineStatusDto {
  active_lane: 'fast' | 'deep' | string;
  fast_timeout_ms: number;
  deep_timeout_ms: number;
  providers: ProviderStatusDto[];
  free_key_offers: FreeTierOffer[];
}

export interface ResearchEngineConfigDto {
  active_lane?: 'fast' | 'deep' | string;
  fast_timeout_ms?: number;
  deep_timeout_ms?: number;
  enabled_providers?: string[];
  provider_api_keys?: Record<string, string>;
}

export async function getResearchEngineStatus(): Promise<ResearchEngineStatusDto> {
  return invoke<ResearchEngineStatusDto>('get_research_engine_status');
}

export async function saveResearchEngineConfig(
  config: ResearchEngineConfigDto,
): Promise<ResearchEngineStatusDto> {
  return invoke<ResearchEngineStatusDto>('save_research_engine_config', { config });
}



