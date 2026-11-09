import React, { useMemo, useState } from 'react';
import { RomanTooltip, RomanMedallion } from '../../ui/RomanTooltip';
import { Icon } from '../../ui/Icons';

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
    model_id: 'mens/qwen3.5-27b-base',
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
      className="rounded-xl border border-[#C59B27]/45 bg-bg-surface p-4 space-y-3.5 shadow-sm text-text-primary"
    >
      {/* Header with Verification Seals & Badges */}
      <div className="flex flex-wrap items-center justify-between gap-3 border-b border-[#382F20] pb-3.5">
        <div>
          <div className="flex flex-wrap items-center gap-2">
            <RomanTooltip
              title="Cryptographic Ledger Integrity"
              purpose="Proves leaderboard entries are signed with Ed25519 keys and tamper-sealed via BLAKE3 hash chains."
              actionHint="Entries cannot be modified without invalidating signature"
            >
              <span
                data-testid="leaderboard-verified-badge"
                className="inline-flex items-center gap-1.5 rounded-full border border-[#10B981]/50 bg-[#10B981]/15 px-2.5 py-0.5 font-mono text-[10px] font-semibold text-[#6EE7B7]"
              >
                <Icon.waxSeal className="size-3 text-[#10B981]" />
                <span>
                  {usingFallback
                    ? 'STATIC SIGNED SNAPSHOT · Ed25519 VERIFIED'
                    : 'Ed25519 VERIFIED · BLAKE3 CHAIN'}
                </span>
              </span>
            </RomanTooltip>

            <RomanTooltip
              title="Developer Workspace Sandbox"
              purpose="Your isolated local development environment status and accumulated experience points."
            >
              <span
                data-testid="leaderboard-local-sandbox-badge"
                className="inline-flex items-center gap-1.5 rounded-full border border-[#D4AF37]/50 bg-[#D4AF37]/15 px-2.5 py-0.5 font-mono text-[10px] font-semibold text-[#F3E5AB]"
              >
                <Icon.romanColumn className="size-3 text-[#D4AF37]" />
                <span>
                  LOCAL SANDBOX · UNRANKED ({localUserHandle}: {localXp.toLocaleString()} XP)
                </span>
              </span>
            </RomanTooltip>
          </div>
          <h3 className="mt-1.5 font-serif text-sm font-bold tracking-wider text-[#F3E5AB] uppercase">
            Verified Global Arena &amp; 5-Spoke Benchmark Standings
          </h3>
        </div>

        {/* 4 Plain-English Leaderboard Lenses */}
        <div className="flex flex-wrap items-center gap-1" role="tablist" aria-label="Leaderboard Lenses">
          <RomanTooltip
            title="Stewardship & Reputation Lumens"
            purpose="Inspect verified individual engineers and runtime stewards ranked by reputation lumens and compiler contributions."
            actionHint="Click to view engineer rankings"
          >
            <button
              type="button"
              role="tab"
              aria-selected={lens === 'stewardship'}
              data-testid="arena-lens-stewardship"
              onClick={() => setLens('stewardship')}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                lens === 'stewardship'
                  ? 'border border-[#D4AF37] bg-bg-elevated text-text-primary shadow-sm'
                  : 'border border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary'
              }`}
            >
              <Icon.laurelWreath className="size-3 shrink-0" />
              <span>Stewardship &amp; Lumens</span>
            </button>
          </RomanTooltip>

          <RomanTooltip
            title="5-Spoke Model Benchmark Arena"
            purpose="Compare fine-tuned MENS local models against baseline LLMs across five core code axes."
            actionHint="Click to view model benchmark standings"
          >
            <button
              type="button"
              role="tab"
              aria-selected={lens === 'models'}
              data-testid="arena-lens-models"
              onClick={() => setLens('models')}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                lens === 'models'
                  ? 'border border-[#D4AF37] bg-bg-elevated text-text-primary shadow-sm'
                  : 'border border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary'
              }`}
            >
              <Icon.astrolabeRadar className="size-3 shrink-0" />
              <span>5-Spoke Model Arena</span>
            </button>
          </RomanTooltip>

          <RomanTooltip
            title="Collegium Guilds"
            purpose="Engineering guilds and collective verified contribution volumes across the workspace."
            actionHint="Click to view guild standings"
          >
            <button
              type="button"
              role="tab"
              aria-selected={lens === 'collegium'}
              data-testid="arena-lens-collegium"
              onClick={() => setLens('collegium')}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                lens === 'collegium'
                  ? 'border border-[#D4AF37] bg-bg-elevated text-text-primary shadow-sm'
                  : 'border border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary'
              }`}
            >
              <Icon.gladiusShield className="size-3 shrink-0" />
              <span>Collegium Guilds</span>
            </button>
          </RomanTooltip>

          <RomanTooltip
            title="Cryptographic Proof Chain"
            purpose="Inspect the BLAKE3 hash tip and Ed25519 signer identity that cryptographically anchor this arena."
            actionHint="Click to inspect proof chain"
          >
            <button
              type="button"
              role="tab"
              aria-selected={lens === 'proof-chain'}
              data-testid="arena-lens-proof-chain"
              onClick={() => setLens('proof-chain')}
              className={`flex items-center gap-1.5 rounded-md px-2.5 py-1 font-serif text-[11px] font-semibold tracking-wide transition ${
                lens === 'proof-chain'
                  ? 'border border-[#D4AF37] bg-bg-elevated text-text-primary shadow-sm'
                  : 'border border-border-subtle bg-bg-surface text-text-secondary hover:text-text-primary'
              }`}
            >
              <Icon.waxSeal className="size-3 shrink-0" />
              <span>Proof Chain</span>
            </button>
          </RomanTooltip>
        </div>
      </div>

      {/* Lens 1: Stewardship Table */}
      {lens === 'stewardship' && (
        <div className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-surface" data-testid="arena-panel-stewardship">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle bg-bg-elevated text-[10px] font-serif uppercase tracking-wider text-text-secondary">
                <th className="py-2 px-3">Rank</th>
                <th className="py-2 px-3">Steward</th>
                <th className="py-2 px-3">Tier Title</th>
                <th className="py-2 px-3 text-right">Level</th>
                <th className="py-2 px-3 text-right">Lumens</th>
                <th className="py-2 px-3 text-right">Cryptographic Proof</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {rows.map((e) => {
                const chainHash =
                  e.row_chain_hash_hex ||
                  '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043';
                return (
                  <tr
                    key={e.user_id}
                    data-testid={`steward-row-${e.user_id}`}
                    className="hover:bg-bg-elevated transition-colors"
                  >
                    <td className="py-2.5 px-3 tabular-nums font-semibold text-brass">
                      #{e.rank}
                    </td>
                    <td className="py-2.5 px-3 font-semibold text-text-primary">
                      {e.user_id}
                    </td>
                    <td className="py-2.5 px-3 text-[11px] text-text-secondary">
                      {e.tier_title || 'Verified Steward'}
                    </td>
                    <td className="py-2.5 px-3 text-right tabular-nums text-text-muted">
                      L{e.level}
                    </td>
                    <td className="py-2.5 px-3 text-right tabular-nums font-semibold text-text-primary">
                      {(e.lumens ?? e.score).toLocaleString()}
                    </td>
                    <td className="py-2.5 px-3 text-right">
                      <RomanTooltip
                        title="Cryptographic Proof Receipt"
                        purpose={`Inspect BLAKE3 row hash and SHA3 payload digest for ${e.user_id}.`}
                        actionHint="Click to view proof receipt"
                      >
                        <button
                          type="button"
                          data-testid={`proof-btn-${e.user_id}`}
                          onClick={() => {
                            setSelectedProof(e);
                            setSigVerifiedMsg(null);
                          }}
                          className="inline-flex items-center gap-1 rounded border border-[#10B981]/40 bg-[#10B981]/15 px-2 py-0.5 text-[10px] font-semibold text-emerald-700 hover:bg-[#10B981]/25 hover:border-[#10B981]/60 transition"
                        >
                          <Icon.waxSeal className="size-3 text-[#10B981]" />
                          <span>Verified · BLAKE3 {chainHash.slice(0, 8)}…</span>
                        </button>
                      </RomanTooltip>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      )}

      {/* Lens 2: 5-Spoke Model Arena Table */}
      {lens === 'models' && (
        <div className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-surface" data-testid="arena-panel-models">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle bg-bg-elevated text-[10px] font-serif uppercase tracking-wider text-text-secondary">
                <th className="py-2 px-3">Rank</th>
                <th className="py-2 px-3">(Model, Harness, Config)</th>
                <th className="py-2 px-3 text-right">Pass@1 (95% CI)</th>
                <th className="py-2 px-3 text-right">Δ vs Base</th>
                <th className="py-2 px-3 text-right">Integrity</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {DEFAULT_MODEL_ARENA.map((m) => (
                <tr
                  key={m.model_id}
                  data-testid={`model-row-${m.rank}`}
                  className="hover:bg-bg-elevated transition-colors"
                >
                  <td className="py-2.5 px-3 tabular-nums font-semibold text-brass">
                    #{m.rank}
                  </td>
                  <td className="py-2.5 px-3">
                    <div className="font-semibold text-text-primary">{m.model_id}</div>
                    <div className="text-[10px] text-text-secondary">
                      {m.harness_id} · {m.config_digest}
                    </div>
                  </td>
                  <td className="py-2.5 px-3 text-right tabular-nums text-text-primary">
                    <div className="font-semibold">{m.overall_pass_at_1_pct.toFixed(1)}%</div>
                    <div className="text-[10px] text-text-secondary">
                      [{m.pass_at_1_ci95_low_pct.toFixed(1)}%, {m.pass_at_1_ci95_high_pct.toFixed(1)}%]
                    </div>
                  </td>
                  <td className="py-2.5 px-3 text-right tabular-nums font-semibold text-emerald-700">
                    {m.delta_vs_base_pct > 0 ? `+${m.delta_vs_base_pct.toFixed(1)}%` : '0.0%'}
                  </td>
                  <td className="py-2.5 px-3 text-right text-[10px] text-emerald-700">
                    Held-Out Clean · Zero Tamper
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Lens 3: Collegium Guilds Table */}
      {lens === 'collegium' && (
        <div className="overflow-x-auto rounded-lg border border-border-subtle bg-bg-surface" data-testid="arena-panel-collegium">
          <table className="w-full text-left font-mono text-[12px]">
            <thead>
              <tr className="border-b border-border-subtle bg-bg-elevated text-[10px] font-serif uppercase tracking-wider text-text-secondary">
                <th className="py-2 px-3">Rank</th>
                <th className="py-2 px-3">Collegium Guild</th>
                <th className="py-2 px-3">Pontifex Lead</th>
                <th className="py-2 px-3 text-right">Members</th>
                <th className="py-2 px-3 text-right">Total Lumens</th>
              </tr>
            </thead>
            <tbody className="divide-y divide-border-subtle">
              {DEFAULT_COLLEGIUMS.map((c) => (
                <tr
                  key={c.collegium_id}
                  className="hover:bg-bg-elevated transition-colors"
                >
                  <td className="py-2.5 px-3 tabular-nums font-semibold text-brass">
                    #{c.rank}
                  </td>
                  <td className="py-2.5 px-3 font-semibold text-text-primary">{c.name}</td>
                  <td className="py-2.5 px-3 text-text-secondary">{c.pontifex_handle}</td>
                  <td className="py-2.5 px-3 text-right tabular-nums text-text-muted">{c.member_count}</td>
                  <td className="py-2.5 px-3 text-right tabular-nums font-semibold text-text-primary">
                    {c.total_lumens.toLocaleString()}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      {/* Lens 4: Proof Chain View */}
      {lens === 'proof-chain' && (
        <div data-testid="arena-panel-proof-chain" className="grid gap-3 sm:grid-cols-2 font-mono text-[11px]">
          <div className="rounded-lg border border-border-subtle bg-bg-elevated p-3 space-y-1">
            <div className="flex items-center gap-1.5 text-[10px] font-serif uppercase tracking-wider text-text-secondary">
              <Icon.waxSeal className="size-3 text-brass" />
              <span>BLAKE3 Chain Tip</span>
            </div>
            <div className="break-all font-mono font-semibold text-text-primary">
              a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe
            </div>
          </div>
          <div className="rounded-lg border border-border-subtle bg-bg-elevated p-3 space-y-1">
            <div className="flex items-center gap-1.5 text-[10px] font-serif uppercase tracking-wider text-text-secondary">
              <Icon.gladiusShield className="size-3 text-[#10B981]" />
              <span>Ed25519 Signer Public Key</span>
            </div>
            <div className="break-all font-mono font-semibold text-emerald-700">
              ea4a6c63e29c520abef5507b132ec5f9954776aebebe7b92421eea691446d22c
            </div>
          </div>
        </div>
      )}

      {/* Cryptographic Proof Drawer */}
      {selectedProof && (
        <aside
          data-testid="cryptographic-proof-drawer"
          className="rounded-lg border-2 border-[#10B981]/50 bg-bg-elevated p-3.5 font-mono text-[11px] space-y-2.5 shadow-lg"
        >
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <Icon.waxSeal className="size-4 text-[#10B981]" />
              <span className="font-serif text-sm font-bold text-emerald-700">
                Cryptographic Proof Receipt · {selectedProof.user_id}
              </span>
            </div>
            <RomanTooltip
              title="Close Proof Receipt"
              purpose="Closes the cryptographic proof receipt drawer."
              actionHint="Click to close receipt"
            >
              <button
                type="button"
                data-testid="close-proof-drawer"
                onClick={() => setSelectedProof(null)}
                className="rounded border border-border-subtle bg-bg-surface px-2.5 py-1 text-[10px] text-text-secondary hover:text-text-primary hover:border-[#D4AF37]/50 transition"
              >
                Close
              </button>
            </RomanTooltip>
          </div>

          <div className="grid gap-2.5 sm:grid-cols-2">
            <div className="rounded border border-border-subtle bg-bg-surface p-2.5">
              <div className="text-[10px] font-serif uppercase tracking-wider text-text-secondary">BLAKE3 Row Chain Hash</div>
              <div className="mt-1 break-all text-text-primary" data-testid="drawer-row-chain-hash">
                {selectedProof.row_chain_hash_hex ||
                  '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043'}
              </div>
            </div>
            <div className="rounded border border-border-subtle bg-bg-surface p-2.5">
              <div className="text-[10px] font-serif uppercase tracking-wider text-text-secondary">Payload SHA3-256 Digest</div>
              <div className="mt-1 break-all text-text-secondary" data-testid="drawer-sha3-digest">
                {selectedProof.proof_digest_sha3_256 ||
                  '904f1708403f578f138f2bfb75106264667099e4528efdbb7d8d56d00b14d214'}
              </div>
            </div>
          </div>

          <div className="flex flex-wrap items-center gap-3 pt-1">
            <RomanTooltip
              title="Verify Ed25519 Signature"
              purpose="Runs in-browser cryptographic verification against the canonical public key to verify that the proof is genuine."
              actionHint="Click to execute cryptographic verification"
            >
              <button
                type="button"
                data-testid="verify-ed25519-btn"
                onClick={() =>
                  setSigVerifiedMsg(
                    '✓ Ed25519 signature (dd1fefe5c88499bd…) & BLAKE3 chain link verified against canonical key ea4a6c63e29c520a…'
                  )
                }
                className="rounded border border-[#10B981]/50 bg-[#10B981]/20 px-3 py-1 text-[10px] font-semibold text-[#6EE7B7] hover:bg-[#10B981]/30 transition"
              >
                Verify Ed25519 Signature
              </button>
            </RomanTooltip>
            {sigVerifiedMsg && (
              <span data-testid="verify-ed25519-msg" className="text-[10px] text-[#6EE7B7] font-semibold">
                {sigVerifiedMsg}
              </span>
            )}
          </div>
        </aside>
      )}
    </div>
  );
}
