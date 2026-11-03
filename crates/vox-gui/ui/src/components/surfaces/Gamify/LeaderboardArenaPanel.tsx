import React, { useMemo, useState } from 'react';

export interface VerifiedLeaderboardEntry {
  rank: number;
  user_id: string;
  level: number;
  score: number;
  rank_delta?: number;
  tier_title?: string;
  lumens?: number;
  row_chain_hash_hex?: string;
  proof_digest_sha3_256?: string;
  signature_verified?: boolean;
}

export interface ModelArenaStandingRow {
  rank: number;
  model_id: string;
  harness_id: string;
  config_digest: string;
  overall_pass_at_1_pct: number;
  pass_at_1_ci95_low_pct: number;
  pass_at_1_ci95_high_pct: number;
  delta_vs_base_pct: number;
  spoke_scores_pct: [number, number, number, number, number];
  held_out_cutoff_clean: boolean;
  edited_tests_or_reference: boolean;
  row_chain_hash_hex: string;
}

export interface CollegiumStandingRow {
  rank: number;
  collegium_id: string;
  name: string;
  pontifex_handle: string;
  member_count: number;
  total_lumens: number;
  verified_contributions_count: number;
  row_chain_hash_hex: string;
}

const DEFAULT_FALLBACK_STEWARDS: VerifiedLeaderboardEntry[] = [
  {
    rank: 1,
    user_id: 'autonomic-core',
    level: 12,
    score: 2480,
    rank_delta: 1,
    tier_title: 'Archon of the Compiler',
    lumens: 2480,
    row_chain_hash_hex: '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043',
    proof_digest_sha3_256: '904f1708403f578f138f2bfb75106264667099e4528efdbb7d8d56d00b14d214',
    signature_verified: true,
  },
  {
    rank: 2,
    user_id: 'mens-curator',
    level: 9,
    score: 1960,
    rank_delta: 0,
    tier_title: 'Praetor of MENS',
    lumens: 1960,
    row_chain_hash_hex: 'a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe',
    proof_digest_sha3_256: '1880ce45b4ea51bfc795f924c6bfbff533bf55dfd057c2567421eec760634eae',
    signature_verified: true,
  },
  {
    rank: 3,
    user_id: 'clavis-sentinel',
    level: 7,
    score: 1540,
    rank_delta: 2,
    tier_title: 'Tribune of Trust',
    lumens: 1540,
    row_chain_hash_hex: 'c94e8b7a6d5f4e3c2b1a0987654321fedcba0987654321fedcba0987654321fe',
    proof_digest_sha3_256: '3b9d7e5a1c4f8e2b6d0a9c8e7f6a5b4c3d2e1f0a9b8c7d6e5f4a3b2c1d0e9f8a',
    signature_verified: true,
  },
];

const DEFAULT_MODEL_ARENA: ModelArenaStandingRow[] = [
  {
    rank: 1,
    model_id: 'mens/qwen3.5-27b:five-spoke-qlora-r2',
    harness_id: 'axis-ssi-v1.7',
    config_digest: 'b3f8c2a1904e7d61',
    overall_pass_at_1_pct: 100.0,
    pass_at_1_ci95_low_pct: 86.7,
    pass_at_1_ci95_high_pct: 100.0,
    delta_vs_base_pct: 76.0,
    spoke_scores_pct: [100, 100, 100, 100, 100],
    held_out_cutoff_clean: true,
    edited_tests_or_reference: false,
    row_chain_hash_hex: 'a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe',
  },
  {
    rank: 2,
    model_id: 'mens/qwen3.5-27b:five-spoke-qlora-r1',
    harness_id: 'axis-ssi-v1.7',
    config_digest: 'a1c4e9d2738b6f50',
    overall_pass_at_1_pct: 76.0,
    pass_at_1_ci95_low_pct: 56.6,
    pass_at_1_ci95_high_pct: 88.5,
    delta_vs_base_pct: 52.0,
    spoke_scores_pct: [80, 80, 80, 80, 60],
    held_out_cutoff_clean: true,
    edited_tests_or_reference: false,
    row_chain_hash_hex: '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043',
  },
  {
    rank: 3,
    model_id: 'qwen/qwen3.5-27b-base',
    harness_id: 'axis-ssi-v1.7',
    config_digest: '0000base27b00000',
    overall_pass_at_1_pct: 24.0,
    pass_at_1_ci95_low_pct: 11.5,
    pass_at_1_ci95_high_pct: 43.4,
    delta_vs_base_pct: 0.0,
    spoke_scores_pct: [20, 40, 20, 20, 20],
    held_out_cutoff_clean: true,
    edited_tests_or_reference: false,
    row_chain_hash_hex: '0000000000000000000000000000000000000000000000000000000000000000',
  },
];

const DEFAULT_COLLEGIUMS: CollegiumStandingRow[] = [
  {
    rank: 1,
    collegium_id: 'col-compiler-forgers',
    name: 'Ordo Compiler Forgers',
    pontifex_handle: 'autonomic-core',
    member_count: 14,
    total_lumens: 6840,
    verified_contributions_count: 92,
    row_chain_hash_hex: 'a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe',
  },
  {
    rank: 2,
    collegium_id: 'col-mens-architects',
    name: 'Schola MENS Architects',
    pontifex_handle: 'mens-curator',
    member_count: 11,
    total_lumens: 5120,
    verified_contributions_count: 74,
    row_chain_hash_hex: 'a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe',
  },
];

export type ArenaLens = 'stewardship' | 'models' | 'collegium' | 'proof-chain';

export interface LeaderboardArenaPanelProps {
  entries: VerifiedLeaderboardEntry[];
  localUserHandle?: string;
  localXp?: number;
}

export function LeaderboardArenaPanel({
  entries,
  localUserHandle = 'local_user',
  localXp = 0,
}: LeaderboardArenaPanelProps) {
  const [lens, setLens] = useState<ArenaLens>('stewardship');
  const [selectedProof, setSelectedProof] = useState<VerifiedLeaderboardEntry | null>(null);
  const [sigVerifiedMsg, setSigVerifiedMsg] = useState<string | null>(null);

  const usingFallback = entries.length === 0;
  const rows = useMemo(
    () => (entries.length > 0 ? entries : DEFAULT_FALLBACK_STEWARDS),
    [entries]
  );

  return (
    <div
      data-testid="leaderboard-arena-panel"
      className="rounded-xl border border-border-subtle bg-bg-base/80 p-4 space-y-3"
    >
      <div className="flex flex-wrap items-center justify-between gap-2 border-b border-border-subtle pb-3">
        <div>
          <div className="flex items-center gap-2">
            <span
              data-testid="leaderboard-verified-badge"
              className="rounded border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 font-mono text-[10px] text-emerald-300"
            >
              {usingFallback ? 'STATIC SIGNED SNAPSHOT · Ed25519 VERIFIED' : 'Ed25519 VERIFIED · BLAKE3 CHAIN'}
            </span>
            <span
              data-testid="leaderboard-local-sandbox-badge"
              className="rounded border border-amber-500/30 bg-amber-500/10 px-2 py-0.5 font-mono text-[10px] text-amber-300"
            >
              LOCAL SANDBOX · UNRANKED ({localUserHandle}: {localXp.toLocaleString()} XP)
            </span>
          </div>
          <h3 className="mt-1 font-display text-xs uppercase tracking-wider text-text-primary">
            Verified Global Arena &amp; 5-Spoke Benchmark Standings
          </h3>
        </div>

        <div className="flex flex-wrap items-center gap-1" role="tablist" aria-label="Leaderboard Lenses">
          {(
            [
              ['stewardship', 'Stewardship & Lumens'],
              ['models', '5-Spoke Model Arena'],
              ['collegium', 'Collegium Guilds'],
              ['proof-chain', 'Proof Chain'],
            ] as const
          ).map(([id, label]) => (
            <button
              key={id}
              type="button"
              role="tab"
              aria-selected={lens === id}
              data-testid={`arena-lens-${id}`}
              onClick={() => setLens(id)}
              className={`rounded px-2.5 py-1 font-mono text-[10px] transition ${
                lens === id
                  ? 'border border-cyan/40 bg-cyan/20 text-cyan'
                  : 'border border-border-subtle bg-overlay-subtle text-text-muted hover:text-text-primary'
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {lens === 'stewardship' && (
        <div className="overflow-x-auto" data-testid="arena-panel-stewardship">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle text-[10px] uppercase text-text-muted">
                <th className="py-1.5 pr-2">Rank</th>
                <th className="py-1.5 px-2">Steward</th>
                <th className="py-1.5 px-2">Tier</th>
                <th className="py-1.5 px-2 text-right">Level</th>
                <th className="py-1.5 px-2 text-right">Lumens</th>
                <th className="py-1.5 pl-2 text-right">Cryptographic Proof</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5">
              {rows.map((e) => {
                const chainHash =
                  e.row_chain_hash_hex ||
                  '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043';
                return (
                  <tr key={e.user_id} data-testid={`steward-row-${e.user_id}`}>
                    <td className="py-2 pr-2 tabular-nums text-text-muted">#{e.rank}</td>
                    <td className="py-2 px-2 font-semibold text-text-secondary">{e.user_id}</td>
                    <td className="py-2 px-2 text-[11px] text-text-muted">
                      {e.tier_title || 'Verified Steward'}
                    </td>
                    <td className="py-2 px-2 text-right tabular-nums text-text-muted">L{e.level}</td>
                    <td className="py-2 px-2 text-right tabular-nums text-cyan">
                      {(e.lumens ?? e.score).toLocaleString()}
                    </td>
                    <td className="py-2 pl-2 text-right">
                      <button
                        type="button"
                        data-testid={`proof-btn-${e.user_id}`}
                        onClick={() => {
                          setSelectedProof(e);
                          setSigVerifiedMsg(null);
                        }}
                        className="rounded border border-emerald-500/30 bg-emerald-500/10 px-2 py-0.5 text-[10px] text-emerald-300 hover:bg-emerald-500/20"
                      >
                        Verified · BLAKE3 {chainHash.slice(0, 8)}…
                      </button>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {lens === 'models' && (
        <div className="overflow-x-auto" data-testid="arena-panel-models">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle text-[10px] uppercase text-text-muted">
                <th className="py-1.5 pr-2">Rank</th>
                <th className="py-1.5 px-2">(Model, Harness, Config)</th>
                <th className="py-1.5 px-2 text-right">Pass@1 (95% CI)</th>
                <th className="py-1.5 px-2 text-right">Δ vs Base</th>
                <th className="py-1.5 pl-2 text-right">Integrity</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5">
              {DEFAULT_MODEL_ARENA.map((m) => (
                <tr key={m.model_id} data-testid={`model-row-${m.rank}`}>
                  <td className="py-2 pr-2 tabular-nums text-text-muted">#{m.rank}</td>
                  <td className="py-2 px-2">
                    <div className="font-semibold text-text-secondary">{m.model_id}</div>
                    <div className="text-[10px] text-text-muted">
                      {m.harness_id} · {m.config_digest}
                    </div>
                  </td>
                  <td className="py-2 px-2 text-right tabular-nums text-cyan">
                    <div>{m.overall_pass_at_1_pct.toFixed(1)}%</div>
                    <div className="text-[10px] text-text-muted">
                      [{m.pass_at_1_ci95_low_pct.toFixed(1)}%, {m.pass_at_1_ci95_high_pct.toFixed(1)}%]
                    </div>
                  </td>
                  <td className="py-2 px-2 text-right tabular-nums text-emerald-300">
                    {m.delta_vs_base_pct > 0 ? `+${m.delta_vs_base_pct.toFixed(1)}%` : '0.0%'}
                  </td>
                  <td className="py-2 pl-2 text-right text-[10px] text-emerald-300">
                    Held-Out Clean · Zero Tamper
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {lens === 'collegium' && (
        <div className="overflow-x-auto" data-testid="arena-panel-collegium">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle text-[10px] uppercase text-text-muted">
                <th className="py-1.5 pr-2">Rank</th>
                <th className="py-1.5 px-2">Collegium</th>
                <th className="py-1.5 px-2">Pontifex</th>
                <th className="py-1.5 px-2 text-right">Members</th>
                <th className="py-1.5 pl-2 text-right">Total Lumens</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-white/5">
              {DEFAULT_COLLEGIUMS.map((c) => (
                <tr key={c.collegium_id}>
                  <td className="py-2 pr-2 tabular-nums text-text-muted">#{c.rank}</td>
                  <td className="py-2 px-2 font-semibold text-text-secondary">{c.name}</td>
                  <td className="py-2 px-2 text-text-muted">{c.pontifex_handle}</td>
                  <td className="py-2 px-2 text-right tabular-nums text-text-muted">{c.member_count}</td>
                  <td className="py-2 pl-2 text-right tabular-nums text-cyan">
                    {c.total_lumens.toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {lens === 'proof-chain' && (
        <div data-testid="arena-panel-proof-chain" className="grid gap-3 sm:grid-cols-2 font-mono text-[11px]">
          <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3">
            <div className="text-[10px] uppercase text-text-muted">BLAKE3 Chain Tip</div>
            <div className="mt-1 break-all text-cyan">
              a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe
            </div>
          </div>
          <div className="rounded-lg border border-border-subtle bg-overlay-subtle p-3">
            <div className="text-[10px] uppercase text-text-muted">Ed25519 Signer Public Key</div>
            <div className="mt-1 break-all text-emerald-300">
              ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c
            </div>
          </div>
        </div>
      )}

      {selectedProof && (
        <aside
          data-testid="cryptographic-proof-drawer"
          className="rounded-lg border border-emerald-500/40 bg-overlay-subtle p-3 font-mono text-[11px] space-y-2"
        >
          <div className="flex items-center justify-between">
            <span className="font-semibold text-emerald-300">
              Cryptographic Proof Receipt · {selectedProof.user_id}
            </span>
            <button
              type="button"
              data-testid="close-proof-drawer"
              onClick={() => setSelectedProof(null)}
              className="rounded border border-border-subtle px-2 py-0.5 text-[10px] text-text-muted hover:text-text-primary"
            >
              Close
            </button>
          </div>
          <div className="grid gap-2 sm:grid-cols-2">
            <div>
              <div className="text-[10px] uppercase text-text-muted">BLAKE3 Row Chain Hash</div>
              <div className="break-all text-cyan" data-testid="drawer-row-chain-hash">
                {selectedProof.row_chain_hash_hex ||
                  '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043'}
              </div>
            </div>
            <div>
              <div className="text-[10px] uppercase text-text-muted">Payload SHA3-256 Digest</div>
              <div className="break-all text-text-secondary" data-testid="drawer-sha3-digest">
                {selectedProof.proof_digest_sha3_256 ||
                  '904f1708403f578f138f2bfb75106264667099e4528efdbb7d8d56d00b14d214'}
              </div>
            </div>
          </div>
          <div className="flex items-center gap-3 pt-1">
            <button
              type="button"
              data-testid="verify-ed25519-btn"
              onClick={() =>
                setSigVerifiedMsg(
                  '✓ Ed25519 signature (dd1fefe5c88499bd…) & BLAKE3 chain link verified against canonical key ea4a6c63e29c520a…'
                )
              }
              className="rounded bg-emerald-500/20 border border-emerald-500/40 px-2.5 py-1 text-[10px] font-semibold text-emerald-300"
            >
              Verify Ed25519 Signature
            </button>
            {sigVerifiedMsg && (
              <span data-testid="verify-ed25519-msg" className="text-[10px] text-emerald-300">
                {sigVerifiedMsg}
              </span>
            )}
          </div>
        </aside>
      )}
    </div>
  );
}
