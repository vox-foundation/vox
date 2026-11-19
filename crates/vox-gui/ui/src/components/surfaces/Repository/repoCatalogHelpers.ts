export interface RepoCapabilitiesDto {
  vox_project?: boolean;
  cargo_workspace?: boolean;
  node_workspace?: boolean;
  python_project?: boolean;
  go_module?: boolean;
  git?: boolean;
  rust?: boolean;
  typescript?: boolean;
  python?: boolean;
  go?: boolean;
  vox?: boolean;
}

export interface ActiveContextDto {
  repository_id: string;
  display_name: string;
  root_path: string;
  access_mode: string;
  read_only: boolean;
}

export interface OpenWikiSummaryDto {
  initialized?: boolean;
  page_count?: number;
  total_pages?: number;
  stale_pages?: string[];
  legacy_mermaid_blocks?: number;
}

export interface RepoStatusPayload {
  root?: string;
  repository_id?: string;
  capabilities?: RepoCapabilitiesDto;
  active_context?: ActiveContextDto;
  openwiki?: OpenWikiSummaryDto;
}

export interface CatalogRepoEntry {
  repository_id?: string | null;
  display_name: string;
  root_path?: string | null;
  resolved_root?: string | null;
  access_mode: string;
  read_only?: boolean;
  resolution_status?: string;
  resolution_error?: string | null;
}

export interface CatalogPayload {
  primary_repository_id?: string | null;
  repositories?: CatalogRepoEntry[];
}

export function detectedStacks(caps?: RepoCapabilitiesDto): string[] {
  if (!caps) return [];
  const stacks: string[] = [];
  if (caps.vox_project || caps.vox) stacks.push('vox');
  if (caps.cargo_workspace || caps.rust) stacks.push('rust');
  if (caps.node_workspace || caps.typescript) stacks.push('typescript');
  if (caps.python_project || caps.python) stacks.push('python');
  if (caps.go_module || caps.go) stacks.push('go');
  if (caps.git) stacks.push('git');
  return stacks;
}

export function extractToolError(
  res: { is_error?: boolean; result?: unknown } | null | undefined,
  fallbackMsg: string,
): string | null {
  if (!res || typeof res !== 'object') return null;
  const inner = res.result as Record<string, unknown> | undefined;
  if (res.is_error) {
    if (inner && typeof inner.error === 'string' && inner.error.trim().length > 0) {
      return inner.error;
    }
    return fallbackMsg;
  }
  if (inner && inner.success === false && typeof inner.error === 'string') {
    return inner.error;
  }
  return null;
}
