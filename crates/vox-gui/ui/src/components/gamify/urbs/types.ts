// crates/vox-gui/ui/src/components/gamify/urbs/types.ts
/** Mirrors the Rust TownScanDto (crates/vox-gui/src/commands/workspace_town.rs). */
export interface TownFile {
  path: string;
  lines: number;
  git_status?: 'M' | 'A' | 'D' | '?' | string;
  diff_added_lines?: number;
  diff_deleted_lines?: number;
}
export interface TownCrate {
  name: string;
  root: string;
  files: TownFile[];
  /** Intra-workspace module dependencies parsed from manifest/imports. */
  deps?: string[];
  dirty_file_count?: number;
  total_churn_lines?: number;
}
export interface TownDependencyEdge {
  from_district_id: string;
  to_district_id: string;
  imported_symbols?: string[];
  call_site_count?: number;
  from_layer?: number;
  to_layer?: number;
  layer_violation?: boolean;
  broken_contract_errors?: number;
}

export interface TownScan {
  crates: TownCrate[];
  /** Cross-module dependency edges with imported symbols and `layers.toml` violation flags. */
  edges?: TownDependencyEdge[];
  /** Absolute workspace root (forward slashes) for open_locator path joins. */
  root: string;
  scanned_at_ms: number;
  truncated: boolean;
  repo_name?: string;
  primary_language?: string;
}

/** Building tier by line-count quartile within its district. */
export type Tier = 0 | 1 | 2 | 3; // hut | villa | insula | temple

export interface BuildingPlot {
  path: string;
  x: number;
  y: number;
  tier: Tier;
  lines?: number;
  /** True when file has uncommitted git changes, active agent edit, or diagnostics. */
  isTouched?: boolean;
  colSpan?: number;
  rowSpan?: number;
  gitStatus?: string;
  diffAdded?: number;
  diffDeleted?: number;
}

export type ArchLayer = 0 | 1 | 2 | 3; // L0 Bedrock | L1 Core | L2 Orchestrator | L3 Surface

export type DistrictArchetype =
  | 'acropolis_engine'
  | 'praetorium_hub'
  | 'harbor_vault'
  | 'amphitheatre_surface'
  | 'library_scholar'
  | 'circus_arena'
  | 'watchtower_leaf'
  | 'insula_forum';

export type RealEstateFocusMode = 'attention' | 'worksite_only' | 'proportional';

export interface DistrictAttentionSignal {
  activeAgentCount?: number;
  dirtyFileCount?: number;
  totalChurnLines?: number;
  errorCount?: number;
  warningCount?: number;
  needsYouGates?: number;
  isSelected?: boolean;
  inBlastRadius?: boolean;
}

export type UrbsLens = 'agents' | 'health' | 'architecture' | 'pipeline' | 'cost';

export interface District {
  name: string;
  /** Tile rect, half-open: [x0, x1) × [y0, y1). */
  x0: number; y0: number; x1: number; y1: number;
  landmark: boolean;
  buildings: BuildingPlot[];
  /** Intra-workspace module dependencies. */
  deps?: string[];
  /** Total lines of code across all files in this module. */
  totalLines?: number;
  /** Architectural elevation layer (0 = L0 Bedrock .. 3 = L3 Surface). */
  layer?: ArchLayer;
  /** Number of workspace modules that depend on this module. */
  fanIn?: number;
  /** Number of workspace modules that this module depends on. */
  fanOut?: number;
  /** Repo-agnostic structural wonder archetype. */
  archetype?: DistrictArchetype;
  /** Dynamic civic title derived from module name + archetype (zero hardcoding). */
  civicTitle?: string;
  /** Real-estate attention multiplier applied during layout. */
  attentionMultiplier?: number;
  /** Number of uncommitted git-modified files in this district. */
  dirtyFileCount?: number;
  /** Total added + deleted lines across uncommitted files in this district. */
  totalChurnLines?: number;
  /** True when district is compacted into a dormant residential summary block. */
  isCompactedDormant?: boolean;
}

export interface TownLayout {
  districts: District[];
  byPath: Record<string, BuildingPlot>;
  /** Cross-module dependency edges between districts (`URBS-VIADUCT-01`). */
  edges?: TownDependencyEdge[];
  /** Interior grid size in tiles (landmarks live outside, see margins). */
  grid: { w: number; h: number };
  /** Row-major w×h road mask (district border tiles). */
  roads: boolean[];
  /** Fixed harness landmark anchors in tile coords (may be outside the grid). */
  landmarks: { castrum: Pt; portus: Pt; aqvae: Pt; gate: Pt };
}
export interface Pt { x: number; y: number }
