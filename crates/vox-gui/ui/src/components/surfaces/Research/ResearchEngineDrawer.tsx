import React, { useEffect, useRef, useState, useCallback } from 'react';
import {
  getResearchEngineStatus,
  saveResearchEngineConfig,
  type ResearchEngineStatusDto,
  type FreeTierOffer,
  type ProviderStatusDto,
} from './researchActions';
import { SafeExternalLink } from './SafeExternalLink';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import type { SurfaceDecoratorProps } from '../decoratorRegistry';

export interface ResearchEngineDrawerProps {
  isOpen: boolean;
  onClose: () => void;
  onConfigSaved?: () => void;
  pushToast?: SurfaceDecoratorProps['pushToast'];
}

const DEFAULT_OFFERS: FreeTierOffer[] = [
  {
    provider_id: 'tavily',
    name: 'Tavily Search',
    signup_url: 'https://app.tavily.com/sign-up',
    free_tier_description: '1,000 queries/month free web search for AI agents and LLMs.',
    quota_summary: '1,000 searches/mo',
    requires_credit_card: false,
    secret_id: 'tavily_api_key',
  },
];

export function ResearchEngineDrawer({
  isOpen,
  onClose,
  onConfigSaved,
  pushToast,
}: ResearchEngineDrawerProps) {
  const [status, setStatus] = useState<ResearchEngineStatusDto | null>(null);
  const [enabledMap, setEnabledMap] = useState<Record<string, boolean>>({});
  const [keyInputs, setKeyInputs] = useState<Record<string, string>>({});
  const [fastTimeout, setFastTimeout] = useState<number>(2500);
  const [deepTimeout, setDeepTimeout] = useState<number>(15000);
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const drawerRef = useRef<HTMLDivElement>(null);

  const loadStatus = useCallback(async () => {
    try {
      const res = await getResearchEngineStatus();
      setStatus(res);
      setFastTimeout(res.fast_timeout_ms);
      setDeepTimeout(res.deep_timeout_ms);
      const initialEnabled: Record<string, boolean> = {};
      for (const p of res.providers) {
        initialEnabled[p.id] = p.is_enabled;
      }
      setEnabledMap(initialEnabled);
    } catch {
      // Offline fallback
    }
  }, []);

  useEffect(() => {
    if (isOpen) {
      loadStatus();
    }
  }, [isOpen, loadStatus]);

  // Escape key handler: stopPropagation and close
  useEffect(() => {
    if (!isOpen) return;
    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.stopPropagation();
        onClose();
      }
    };
    document.addEventListener('keydown', handleKeyDown);
    return () => document.removeEventListener('keydown', handleKeyDown);
  }, [isOpen, onClose]);

  // Circular focus trap
  useEffect(() => {
    if (!isOpen) return;
    const handleTrap = (e: KeyboardEvent) => {
      if (e.key !== 'Tab' || !drawerRef.current) return;
      const focusable = drawerRef.current.querySelectorAll<HTMLElement>(
        'button, [href], input, select, textarea, [tabindex]:not([tabindex="-1"])'
      );
      if (focusable.length === 0) return;
      const first = focusable[0];
      const last = focusable[focusable.length - 1];
      if (e.shiftKey) {
        if (document.activeElement === first) {
          e.preventDefault();
          last.focus();
        }
      } else {
        if (document.activeElement === last) {
          e.preventDefault();
          first.focus();
        }
      }
    };
    document.addEventListener('keydown', handleTrap);
    return () => document.removeEventListener('keydown', handleTrap);
  }, [isOpen]);

  const handleSaveKey = async (providerId: string) => {
    const key = keyInputs[providerId]?.trim();
    if (!key) return;
    setSaving(true);
    try {
      await saveResearchEngineConfig({
        provider_api_keys: { [providerId]: key },
      });
      setKeyInputs((prev) => ({ ...prev, [providerId]: '' }));
      pushToast?.({
        tone: 'ok',
        title: 'Key Stored',
        body: `API key for ${providerId} stored securely in Clavis vault`,
        cause: 'backend-ok',
      });
      await loadStatus();
      onConfigSaved?.();
    } catch (err) {
      pushToast?.({
        tone: 'warn',
        title: 'Key Storage Failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setSaving(false);
    }
  };

  const handleSaveSettings = async () => {
    setSaving(true);
    try {
      const enabledProviders = Object.entries(enabledMap)
        .filter(([, v]) => v)
        .map(([k]) => k);
      await saveResearchEngineConfig({
        fast_timeout_ms: fastTimeout,
        deep_timeout_ms: deepTimeout,
        enabled_providers: enabledProviders,
      });
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2500);
      pushToast?.({
        tone: 'ok',
        title: 'Engine Configuration Saved',
        body: 'Multi-source research policies and timeouts updated',
        cause: 'backend-ok',
      });
      await loadStatus();
      onConfigSaved?.();
    } catch (err) {
      pushToast?.({
        tone: 'warn',
        title: 'Save Failed',
        body: sanitizeErrorForToast(err),
        cause: 'backend-error',
      });
    } finally {
      setSaving(false);
    }
  };

  if (!isOpen) return null;

  const offers = status?.free_key_offers?.length ? status.free_key_offers : DEFAULT_OFFERS;

  return (
    <div
      data-testid="research-engine-drawer-overlay"
      className="fixed inset-0 z-50 flex justify-end bg-black/60 backdrop-blur-xs transition-opacity"
      role="presentation"
    >
      <div
        ref={drawerRef}
        data-testid="research-engine-drawer"
        role="dialog"
        aria-modal="true"
        aria-label="Research Engines & Free Keys"
        className="relative h-full w-full max-w-md border-l border-border-subtle bg-bg-base p-6 shadow-2xl overflow-y-auto space-y-6"
      >
        {/* Header */}
        <div className="flex items-start justify-between border-b border-border-subtle pb-4">
          <div>
            <h2 className="font-display text-lg font-semibold text-text-primary tracking-wide">
              Research Engines &amp; Keys
            </h2>
            <p className="mt-1 text-xs text-text-muted leading-relaxed">
              Manage search providers, free monthly API quotas, and lane timeouts.
            </p>
          </div>
          <button
            type="button"
            data-testid="close-drawer-btn"
            onClick={onClose}
            aria-label="Close drawer"
            className="rounded p-1 text-text-muted hover:text-text-primary hover:bg-overlay-subtle transition-colors text-lg leading-none"
          >
            ✕
          </button>
        </div>

        {/* Zero-Key Guarantee Banner */}
        <div
          data-testid="zero-key-guarantee"
          className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-4 space-y-2"
        >
          <div className="flex items-center gap-2">
            <span className="text-emerald-400 font-bold text-sm">✓ Zero-Key Guarantee</span>
            <span className="rounded bg-emerald-500/20 px-2 py-0.5 text-[10px] font-mono text-emerald-300">
              Built-In
            </span>
          </div>
          <p className="text-xs text-emerald-200/90 leading-relaxed">
            Vox includes full out-of-the-box keyless research. Wikipedia, OpenAlex (academic papers),
            arXiv (scientific preprints), and SearXNG meta-search run with zero keys required and zero
            tracking.
          </p>
          <div className="flex flex-wrap gap-1.5 pt-1 text-[10px] font-mono text-emerald-400">
            <span className="rounded border border-emerald-500/20 bg-black/30 px-2 py-0.5">Wikipedia</span>
            <span className="rounded border border-emerald-500/20 bg-black/30 px-2 py-0.5">OpenAlex</span>
            <span className="rounded border border-emerald-500/20 bg-black/30 px-2 py-0.5">arXiv</span>
            <span className="rounded border border-emerald-500/20 bg-black/30 px-2 py-0.5">SearXNG</span>
          </div>
        </div>

        {/* Free API Key Acquisition Hub */}
        <div className="space-y-3">
          <div className="border-b border-border-subtle pb-1">
            <h3 className="font-display text-sm font-semibold text-text-primary uppercase tracking-wider">
              Free API Key Hub
            </h3>
            <p className="text-[11px] text-text-muted">
              Enhance deep search coverage with free-tier keys. Keys are stored locally in Clavis.
            </p>
          </div>

          <div className="space-y-3">
            {offers.map((offer) => {
              const matchedProvider = status?.providers.find((p) => p.id === offer.provider_id);
              const hasKey = matchedProvider?.has_key ?? false;
              const quota = matchedProvider?.quota_usage;
              const remaining = quota ? Math.max(0, quota.units_limit - quota.units_spent) : null;

              return (
                <div
                  key={offer.provider_id}
                  data-testid={`offer-card-${offer.provider_id}`}
                  className="rounded-xl border border-border-subtle bg-overlay-subtle p-4 space-y-3"
                >
                  <div className="flex items-start justify-between">
                    <div>
                      <div className="font-semibold text-sm text-text-primary flex items-center gap-1.5">
                        {offer.name}
                        {hasKey && (
                          <span className="rounded border border-brass/40 bg-brass/10 px-1.5 py-0.5 text-[10px] font-mono text-brass">
                            Active
                          </span>
                        )}
                      </div>
                      <div className="text-[11px] text-brass/90 font-medium mt-0.5">
                        {offer.quota_summary}
                        {!offer.requires_credit_card && ' · No credit card required'}
                      </div>
                    </div>
                    <SafeExternalLink
                      url={offer.signup_url}
                      className="inline-flex items-center gap-1 rounded border border-brass/40 bg-brass/10 hover:bg-brass/20 text-brass px-2.5 py-1 text-[11px] font-medium transition-colors"
                    >
                      Claim Free Key ↗
                    </SafeExternalLink>
                  </div>

                  <p className="text-xs text-text-secondary leading-relaxed">
                    {offer.free_tier_description}
                  </p>

                  {quota && (
                    <div className="text-[11px] font-mono text-text-muted">
                      Usage this month:{' '}
                      <span className="text-text-primary">
                        {quota.units_spent} / {quota.units_limit}
                      </span>
                      {remaining !== null && (
                        <span className="text-brass ml-1">({remaining} remaining)</span>
                      )}
                    </div>
                  )}

                  <div className="flex gap-2 pt-1">
                    <input
                      type="password"
                      placeholder={hasKey ? '•••••••••••••••• (Key Configured)' : 'Paste API key here…'}
                      value={keyInputs[offer.provider_id] ?? ''}
                      onChange={(e) =>
                        setKeyInputs((prev) => ({ ...prev, [offer.provider_id]: e.target.value }))
                      }
                      className="flex-1 rounded-lg border border-border-subtle bg-black/40 px-3 py-1.5 text-xs text-text-primary outline-none focus:border-brass/50 font-mono"
                    />
                    <button
                      type="button"
                      disabled={saving || !keyInputs[offer.provider_id]?.trim()}
                      onClick={() => handleSaveKey(offer.provider_id)}
                      className="rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-1.5 text-xs font-medium text-text-secondary hover:text-text-primary hover:border-brass/40 transition-colors disabled:opacity-40"
                    >
                      Save Key
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        </div>

        {/* Engine Toggles & Lane Policy */}
        <div className="space-y-3">
          <div className="border-b border-border-subtle pb-1">
            <h3 className="font-display text-sm font-semibold text-text-primary uppercase tracking-wider">
              Search Engines &amp; Timeouts
            </h3>
            <p className="text-[11px] text-text-muted">
              Enable or disable specific engines and adjust timeout constraints per lane.
            </p>
          </div>

          <div className="space-y-2">
            {status?.providers.map((p) => (
              <div
                key={p.id}
                className="flex items-center justify-between rounded-lg border border-border-subtle bg-overlay-subtle px-3 py-2 text-xs"
              >
                <div>
                  <span className="font-medium text-text-primary">{p.name}</span>
                  <span className="text-[10px] text-text-muted ml-2 font-mono">
                    {p.is_keyless ? 'Keyless' : p.has_key ? 'Key active' : 'Key missing'}
                  </span>
                </div>
                <input
                  type="checkbox"
                  checked={enabledMap[p.id] ?? p.is_enabled}
                  onChange={(e) =>
                    setEnabledMap((prev) => ({ ...prev, [p.id]: e.target.checked }))
                  }
                  className="rounded border-border-subtle bg-black/40 text-brass focus:ring-brass/40 size-4"
                />
              </div>
            ))}
          </div>

          <div className="grid grid-cols-2 gap-3 pt-2">
            <div>
              <label className="block text-[11px] font-medium text-text-secondary mb-1">
                Fast Lane Timeout (ms)
              </label>
              <input
                type="number"
                value={fastTimeout}
                onChange={(e) => setFastTimeout(Number(e.target.value))}
                min={500}
                max={10000}
                step={250}
                className="w-full rounded-lg border border-border-subtle bg-black/40 px-3 py-1.5 text-xs text-text-primary outline-none focus:border-brass/50 font-mono"
              />
            </div>
            <div>
              <label className="block text-[11px] font-medium text-text-secondary mb-1">
                Deep Lane Timeout (ms)
              </label>
              <input
                type="number"
                value={deepTimeout}
                onChange={(e) => setDeepTimeout(Number(e.target.value))}
                min={5000}
                max={60000}
                step={1000}
                className="w-full rounded-lg border border-border-subtle bg-black/40 px-3 py-1.5 text-xs text-text-primary outline-none focus:border-brass/50 font-mono"
              />
            </div>
          </div>
        </div>

        {/* Footer Actions */}
        <div className="flex items-center justify-between pt-4 border-t border-border-subtle">
          {saveSuccess ? (
            <span className="text-xs text-emerald-400 font-medium">✓ Configuration Saved</span>
          ) : (
            <span />
          )}
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={onClose}
              className="rounded-lg border border-border-subtle px-3 py-1.5 text-xs text-text-muted hover:text-text-secondary transition-colors"
            >
              Cancel
            </button>
            <button
              type="button"
              disabled={saving}
              onClick={handleSaveSettings}
              className="rounded-lg border border-brass/40 bg-brass/20 hover:bg-brass/30 text-brass px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-50"
            >
              {saving ? 'Saving…' : 'Save Policies'}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
