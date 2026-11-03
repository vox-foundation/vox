import React from 'react';
import type { BrowserPageInfo, BrowserPageSummary } from '../../../transport';
import type { LaunchMode } from './launchMode';

export type ControlMode = 'you' | 'agent';

const MODE_BUTTON =
  'rounded-lg px-3 py-1.5 text-[11px] uppercase tracking-wider disabled:opacity-50';
const MODE_ACTIVE = 'bg-brass/15 text-brass ring-1 ring-brass/30';
const MODE_IDLE = 'bg-overlay-subtle text-text-muted';

export interface BrowserToolbarProps {
  agentUrl: string;
  onAgentUrlChange: (url: string) => void;
  headless: boolean;
  onHeadlessChange: (value: boolean) => void;
  launchMode: LaunchMode;
  onLaunchModeChange: (mode: LaunchMode) => void;
  profileId: string;
  onProfileIdChange: (id: string) => void;
  saveProfile: boolean;
  onSaveProfileChange: (value: boolean) => void;
  cdpUrl: string;
  onCdpUrlChange: (url: string) => void;
  busy: boolean;
  pageId: string | null;
  pageInfo: BrowserPageInfo | null;
  pages: BrowserPageSummary[];
  agentNavUrl: string;
  onAgentNavUrlChange: (url: string) => void;
  controlMode: ControlMode;
  onOpen: () => void;
  onClose: () => void;
  onCapture: () => void;
  onNavigate: (action: 'back' | 'forward' | 'reload' | 'stop') => void;
  onGoto: () => void;
  onAttach: (pageId: string) => void;
  onClosePage: (pageId: string) => void;
  onControlModeToggle: () => void;
}

export function BrowserToolbar({
  agentUrl,
  onAgentUrlChange,
  headless,
  onHeadlessChange,
  launchMode,
  onLaunchModeChange,
  profileId,
  onProfileIdChange,
  saveProfile,
  onSaveProfileChange,
  cdpUrl,
  onCdpUrlChange,
  busy,
  pageId,
  pageInfo,
  pages,
  agentNavUrl,
  onAgentNavUrlChange,
  controlMode,
  onOpen,
  onClose,
  onCapture,
  onNavigate,
  onGoto,
  onAttach,
  onClosePage,
  onControlModeToggle,
}: BrowserToolbarProps) {
  return (
    <div className="space-y-3">
      <div className="flex flex-wrap gap-2" role="group" aria-label="Launch mode">
        <button
          type="button"
          aria-pressed={launchMode === 'ephemeral'}
          onClick={() => onLaunchModeChange('ephemeral')}
          className={`${MODE_BUTTON} ${launchMode === 'ephemeral' ? MODE_ACTIVE : MODE_IDLE}`}
        >
          Ephemeral
        </button>
        <button
          type="button"
          aria-pressed={launchMode === 'named'}
          onClick={() => onLaunchModeChange('named')}
          className={`${MODE_BUTTON} ${launchMode === 'named' ? MODE_ACTIVE : MODE_IDLE}`}
        >
          Named
        </button>
        <button
          type="button"
          aria-pressed={launchMode === 'connect-chrome'}
          onClick={() => onLaunchModeChange('connect-chrome')}
          className={`${MODE_BUTTON} ${launchMode === 'connect-chrome' ? MODE_ACTIVE : MODE_IDLE}`}
        >
          Connect Chrome
        </button>
      </div>
      <div className="grid gap-3 md:grid-cols-[1fr_auto]">
        <label className="block space-y-1">
          <span className="text-[10px] uppercase tracking-wider text-text-muted">Agent browser URL</span>
          <input
            value={agentUrl}
            onChange={(e) => onAgentUrlChange(e.target.value)}
            className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
          />
        </label>
        <label className="flex items-end gap-2 pb-1">
          <input
            type="checkbox"
            checked={headless}
            onChange={(e) => onHeadlessChange(e.target.checked)}
            className="rounded-sm"
          />
          <span className="text-[11px] text-text-muted">Headless</span>
        </label>
      </div>
      {launchMode === 'named' && (
        <label className="block space-y-1">
          <span className="text-[10px] uppercase tracking-wider text-text-muted">Profile id</span>
          <input
            value={profileId}
            onChange={(e) => onProfileIdChange(e.target.value)}
            className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
            placeholder="staging-1"
          />
        </label>
      )}
      {launchMode === 'connect-chrome' && (
        <label className="block space-y-1">
          <span className="text-[10px] uppercase tracking-wider text-text-muted">CDP URL (loopback)</span>
          <input
            value={cdpUrl}
            onChange={(e) => onCdpUrlChange(e.target.value)}
            className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
            placeholder="http://127.0.0.1:9222"
          />
        </label>
      )}
      {launchMode !== 'ephemeral' && (
        <label className="flex items-center gap-2">
          <input
            type="checkbox"
            checked={saveProfile}
            onChange={(e) => onSaveProfileChange(e.target.checked)}
            className="rounded-sm"
          />
          <span className="text-[11px] text-text-muted">
            Save cookies and site data for this profile
          </span>
        </label>
      )}
      {launchMode !== 'ephemeral' && (
        <p className="text-[11px] text-text-muted">
          Set VOX_BROWSER_ALLOWED_HOSTS in production. Consent is stored only in MCP consents.json.
        </p>
      )}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          disabled={busy}
          onClick={onOpen}
          className="rounded-lg bg-brass/20 text-brass px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Open session
        </button>
        <button
          type="button"
          disabled={busy || !pageId}
          onClick={onClose}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Close
        </button>
        <button
          type="button"
          disabled={!pageId || busy}
          onClick={onCapture}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Capture frame
        </button>
        <button
          type="button"
          disabled={!pageId || busy || !(pageInfo?.can_go_back ?? false)}
          onClick={() => onNavigate('back')}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Back
        </button>
        <button
          type="button"
          disabled={!pageId || busy || !(pageInfo?.can_go_forward ?? false)}
          onClick={() => onNavigate('forward')}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Forward
        </button>
        <button
          type="button"
          disabled={!pageId || busy}
          onClick={() => onNavigate('reload')}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Reload
        </button>
        <button
          type="button"
          disabled={!pageId || busy}
          onClick={() => onNavigate('stop')}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Stop
        </button>
        <button
          type="button"
          aria-pressed={controlMode === 'agent'}
          onClick={onControlModeToggle}
          className={`rounded-lg px-4 py-2 text-[11px] uppercase tracking-wider ${
            controlMode === 'you'
              ? 'bg-brass/20 text-brass'
              : 'bg-overlay-subtle text-text-secondary'
          }`}
        >
          Mode: {controlMode === 'you' ? 'You' : 'Agent'}
        </button>
      </div>
      <div className="grid gap-2 md:grid-cols-[1fr_auto]">
        <input
          value={agentNavUrl}
          onChange={(e) => onAgentNavUrlChange(e.target.value)}
          className="w-full rounded-lg bg-overlay-subtle border border-border-subtle px-3 py-2 text-sm text-text-secondary"
          placeholder="https://example.com"
        />
        <button
          type="button"
          disabled={!pageId || busy || !agentNavUrl.trim()}
          onClick={onGoto}
          className="rounded-lg bg-overlay-subtle text-text-secondary px-4 py-2 text-[11px] uppercase tracking-wider disabled:opacity-50"
        >
          Go
        </button>
      </div>
      <div className="flex flex-wrap gap-2">
        {pages.map((p) => {
          const active = p.page_id === pageId;
          return (
            <div
              key={p.page_id}
              className={`rounded-lg px-3 py-1.5 text-[11px] max-w-[280px] truncate ${
                active
                  ? 'bg-brass/20 text-brass border border-brass/40'
                  : 'bg-overlay-subtle text-text-secondary border border-border-subtle'
              }`}
            >
              <button
                type="button"
                onClick={() => onAttach(p.page_id)}
                className="mr-2 max-w-[220px] truncate align-middle"
                title={`${p.title || '(untitled)'} — ${p.url}`}
              >
                {(p.title || '(untitled)').slice(0, 42)}
              </button>
              <button
                type="button"
                onClick={() => onClosePage(p.page_id)}
                className="align-middle text-text-muted hover:text-text-primary"
                aria-label={`Close ${p.title || p.page_id}`}
                title="Close tab"
              >
                ×
              </button>
            </div>
          );
        })}
      </div>
      <p className="text-[11px] text-text-muted font-mono">
        page_id={pageId ?? '—'} · can_go_back={String(pageInfo?.can_go_back ?? false)} · can_go_forward={String(pageInfo?.can_go_forward ?? false)}
      </p>
    </div>
  );
}
