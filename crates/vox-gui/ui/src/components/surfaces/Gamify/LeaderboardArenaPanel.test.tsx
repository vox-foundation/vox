import React from 'react';
import { fireEvent, render, screen } from '@testing-library/react';
import { describe, expect, it } from 'vitest';
import { LeaderboardArenaPanel } from './LeaderboardArenaPanel';

describe('LeaderboardArenaPanel', () => {
  it('renders offline signed snapshot fallback when entries list is empty', () => {
    render(<LeaderboardArenaPanel entries={[]} localUserHandle="dev_local" localXp={420} />);
    expect(screen.getByTestId('leaderboard-verified-badge').textContent).toContain(
      'STATIC SIGNED SNAPSHOT · Ed25519 VERIFIED'
    );
    expect(screen.getByTestId('leaderboard-local-sandbox-badge').textContent).toContain(
      'LOCAL SANDBOX · UNRANKED'
    );
    expect(screen.getByTestId('steward-row-autonomic-core')).toBeTruthy();
  });

  it('switches across the 4 leaderboard lenses (Stewardship, 5-Spoke Models, Collegium, Proof Chain)', () => {
    render(<LeaderboardArenaPanel entries={[]} />);
    expect(screen.getByTestId('arena-panel-stewardship')).toBeTruthy();

    fireEvent.click(screen.getByTestId('arena-lens-models'));
    expect(screen.getByTestId('arena-panel-models')).toBeTruthy();
    expect(screen.getByTestId('model-row-1').textContent).toContain(
      'mens/qwen3.5-27b:five-spoke-qlora-r2'
    );

    fireEvent.click(screen.getByTestId('arena-lens-collegium'));
    expect(screen.getByTestId('arena-panel-collegium').textContent).toContain(
      'Ordo Compiler Forgers'
    );

    fireEvent.click(screen.getByTestId('arena-lens-proof-chain'));
    expect(screen.getByTestId('arena-panel-proof-chain').textContent).toContain(
      'a11f3d231163b9522d9a298f263d82c828ab71a66c63c9d08b6a50b1c8622efe'
    );
  });

  it('opens the Cryptographic Proof Drawer and verifies Ed25519 signature', () => {
    render(
      <LeaderboardArenaPanel
        entries={[
          {
            rank: 1,
            user_id: 'autonomic-core',
            level: 12,
            score: 2480,
            lumens: 2480,
            row_chain_hash_hex:
              '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043',
            proof_digest_sha3_256:
              '904f1708403f578f138f2bfb75106264667099e4528efdbb7d8d56d00b14d214',
            signature_verified: true,
          },
        ]}
      />
    );

    fireEvent.click(screen.getByTestId('proof-btn-autonomic-core'));
    expect(screen.getByTestId('cryptographic-proof-drawer')).toBeTruthy();
    expect(screen.getByTestId('drawer-row-chain-hash').textContent).toBe(
      '7d4daab4495ef5ce7b2a24c4df28845a0fefa7d97973b8ff76a84a74e687b043'
    );

    fireEvent.click(screen.getByTestId('verify-ed25519-btn'));
    expect(screen.getByTestId('verify-ed25519-msg').textContent).toContain(
      'Ed25519 signature'
    );
  });
});
