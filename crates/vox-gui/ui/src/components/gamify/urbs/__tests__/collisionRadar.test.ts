import { describe, expect, it } from 'vitest';
import { computeAgentWriteSetCones } from '../collisionRadar';

describe('computeAgentWriteSetCones (Phase 272 — URBS-COLLISION-01..04)', () => {
  it('detects direct file write-set conflicts and 1-hop blast-radius district overlaps', () => {
    const reverseDepsByDistrict = new Map<string, readonly string[]>([
      ['core-compiler', ['agent-orchestrator', 'studio-gui']],
      ['agent-orchestrator', ['studio-gui']],
      ['studio-gui', []],
    ]);

    const result = computeAgentWriteSetCones(
      [
        {
          agentId: 'LEGIO-01',
          label: 'LEGIO-01',
          districtId: 'core-compiler',
          targetFiles: ['crates/core-compiler/src/parser.rs'],
        },
        {
          agentId: 'LEGIO-02',
          label: 'LEGIO-02',
          districtId: 'agent-orchestrator',
          targetFiles: ['crates/agent-orchestrator/src/models/registry.rs'],
        },
      ],
      reverseDepsByDistrict,
    );

    expect(result.cones).toHaveLength(2);
    // LEGIO-01 touches core-compiler, whose 1-hop reverse dep includes agent-orchestrator (where LEGIO-02 is writing)
    const overlap = result.collisions.find((c) => c.districtId === 'agent-orchestrator');
    expect(overlap).toBeDefined();
    expect(overlap?.agentA).toBe('LEGIO-01');
    expect(overlap?.agentB).toBe('LEGIO-02');
    expect(overlap?.severity).toBe('blast_radius_overlap');
  });

  it('flags direct file collision when two agents edit the exact same file', () => {
    const result = computeAgentWriteSetCones(
      [
        {
          agentId: 'LEGIO-01',
          label: 'LEGIO-01',
          districtId: 'core',
          targetFiles: ['src/lib.rs'],
        },
        {
          agentId: 'LEGIO-02',
          label: 'LEGIO-02',
          districtId: 'core',
          targetFiles: ['src/lib.rs'],
        },
      ],
      new Map([['core', []]]),
    );

    expect(result.collisions).toHaveLength(1);
    expect(result.collisions[0]).toMatchObject({
      districtId: 'core',
      severity: 'direct_write_conflict',
      overlappingFilePaths: ['src/lib.rs'],
    });
  });
});
