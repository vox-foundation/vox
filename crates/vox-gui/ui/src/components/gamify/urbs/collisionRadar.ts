/**
 * Multi-Agent Write-Set Collision & Blast-Radius Cone Radar (*Concursus Legionum*, `Phase 272` — `URBS-COLLISION-01..04`).
 * Computes each active agent's claimed write-set (`targetFiles`), 1-hop downstream blast-radius modules,
 * and pairwise `CollisionZone` overlaps where two concurrent agents risk clobbering the same file or module contract.
 */

export interface AgentTaskCollisionInput {
  readonly agentId: string;
  readonly label: string;
  readonly districtId: string;
  readonly targetFiles: readonly string[];
  readonly colorHex?: string;
}

export interface AgentWriteSetCone {
  readonly agentId: string;
  readonly label: string;
  readonly primaryDistrictId: string;
  readonly claimedFiles: readonly string[];
  readonly blastRadiusDistrictIds: readonly string[];
  readonly colorHex: string;
}

export interface CollisionZone {
  readonly id: string;
  readonly districtId: string;
  readonly overlappingFilePaths: readonly string[];
  readonly agentA: string;
  readonly agentB: string;
  readonly agentALabel: string;
  readonly agentBLabel: string;
  readonly severity: 'direct_write_conflict' | 'blast_radius_overlap';
}

export interface CollisionRadarSnapshot {
  readonly cones: readonly AgentWriteSetCone[];
  readonly collisions: readonly CollisionZone[];
}

const AGENT_CONE_PALETTE = [
  '#38BDF8', // Cyan surveyor beam
  '#F59E0B', // Amber surveyor beam
  '#A855F7', // Imperial purple beam
  '#10B981', // Emerald beam
];

/**
 * Compute per-agent write-set cones and pairwise collision zones across districts.
 */
export function computeAgentWriteSetCones(
  agentTasks: readonly AgentTaskCollisionInput[],
  reverseDependentsByDistrict: ReadonlyMap<string, readonly string[]>,
): CollisionRadarSnapshot {
  const cones: AgentWriteSetCone[] = agentTasks.map((task, idx) => {
    const revDeps = reverseDependentsByDistrict.get(task.districtId) ?? [];
    return {
      agentId: task.agentId,
      label: task.label,
      primaryDistrictId: task.districtId,
      claimedFiles: task.targetFiles,
      blastRadiusDistrictIds: [task.districtId, ...revDeps],
      colorHex: task.colorHex ?? AGENT_CONE_PALETTE[idx % AGENT_CONE_PALETTE.length],
    };
  });

  const collisions: CollisionZone[] = [];
  for (let i = 0; i < cones.length; i++) {
    for (let j = i + 1; j < cones.length; j++) {
      const a = cones[i];
      const b = cones[j];

      const aFiles = new Set(a.claimedFiles.map((f) => f.replace(/\\/g, '/')));
      const sharedFiles = b.claimedFiles
        .map((f) => f.replace(/\\/g, '/'))
        .filter((f) => aFiles.has(f));

      if (sharedFiles.length > 0 || a.primaryDistrictId === b.primaryDistrictId) {
        collisions.push({
          id: `collision:${a.agentId}:${b.agentId}:${a.primaryDistrictId}`,
          districtId: a.primaryDistrictId,
          overlappingFilePaths:
            sharedFiles.length > 0
              ? sharedFiles
              : [...a.claimedFiles.slice(0, 1), ...b.claimedFiles.slice(0, 1)],
          agentA: a.agentId,
          agentB: b.agentId,
          agentALabel: a.label,
          agentBLabel: b.label,
          severity: 'direct_write_conflict',
        });
        continue;
      }

      const aBlast = new Set(a.blastRadiusDistrictIds);
      const sharedDistricts = b.blastRadiusDistrictIds.filter((d) => aBlast.has(d));
      if (sharedDistricts.length > 0) {
        const conflictDistrict = sharedDistricts[0];
        collisions.push({
          id: `collision:${a.agentId}:${b.agentId}:${conflictDistrict}`,
          districtId: conflictDistrict,
          overlappingFilePaths: [
            ...(a.claimedFiles.slice(0, 1) || []),
            ...(b.claimedFiles.slice(0, 1) || []),
          ],
          agentA: a.agentId,
          agentB: b.agentId,
          agentALabel: a.label,
          agentBLabel: b.label,
          severity: 'blast_radius_overlap',
        });
      }
    }
  }

  return { cones, collisions };
}
