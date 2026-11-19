import React, { useCallback, useEffect, useMemo, useRef, useState } from 'react';
import { useVoxQuery } from '../../../hooks/useVoxQuery';
import { BackendUnavailableError, sanitizeErrorForToast } from '../../../lib/backendGuard';
import { unwrapMcpEnvelope, type McpInvokeResult } from '../../../lib/mcpToolResult';
import { voxTransport } from '../../../transport';
import { Button } from '../../ui/Button';
import {
  ResearchReportMarkdown,
  detectDocFormat,
  extractDocHeadings,
  listDocFormatCycle,
  type DocFormat,
} from '../Research/ResearchReportMarkdown';
import { EmpiricalVerificationBadge } from './EmpiricalVerificationBadge';
import { OpenWikiHealStudio } from './OpenWikiHealStudio';
import {
  DocReaderHealthPanel,
  DocReaderInboxPanel,
  DocReaderLibraryBar,
} from './DocReaderPanels';
import {
  PROVENANCE_REGEX,
  mapInboxFindings,
  mapKbSearchHits,
  resolveDocStatusInfo,
  type KbFindingItem,
  type KbLibraryTab,
  type KbPromotionScope,
  type KbSearchResultItem,
  type KbStatusDto,
} from './docReaderHelpers';

export type { KbLibraryTab, KbFindingItem, KbSearchResultItem } from './docReaderHelpers';

type DocWorkspaceMode = 'read' | 'split' | 'edit';

interface DocReaderProps {
  path: string;
  initialContent?: string;
  initialMode?: DocWorkspaceMode;
  onNavigateToResearch?: (sessionId: number) => void;
  onSelectDocPath?: (nextPath: string) => void;
}

const NEW_DOC_TEMPLATE = [
  '---',
  'title: "New Workspace Note"',
  'description: "Structured technical specification or empirical research note."',
  'category: "Architecture SSOTs"',
  'status: "research"',
  'topic: "scientia-research"',
  'domain: "knowledge"',
  'authorship: "human"',
  '---',
  '',
  '# New Workspace Note',
  '',
  '<!-- kb:summary -->',
  'Summarize key architectural invariants, findings, and verification evidence in 40 to 120 words.',
  '<!-- /kb:summary -->',
  '',
  '## Findings & Notes',
  '',
  '- [ ] Link primary code or specification citations',
].join('\n');

export function DocReader({
  path,
  initialContent,
  initialMode,
  onNavigateToResearch,
  onSelectDocPath,
}: DocReaderProps) {
  const [activeTab, setActiveTab] = useState<KbLibraryTab>('reader');
  const [activePath, setActivePath] = useState<string>(path);
  const [searchQuery, setSearchQuery] = useState<string>('');
  const [topicFilter, setTopicFilter] = useState<string>('all');
  const [domainFilter, setDomainFilter] = useState<string>('all');
  const [trustFilter, setTrustFilter] = useState<string>('all');
  const [promoteScope, setPromoteScope] = useState<KbPromotionScope>('repo_local');
  const [libraryItems, setLibraryItems] = useState<KbSearchResultItem[]>([]);
  const [inboxItems, setInboxItems] = useState<KbFindingItem[]>([]);
  const [kbStatus, setKbStatus] = useState<KbStatusDto | null>(null);
  const [staleVerifiedCount, setStaleVerifiedCount] = useState<number>(0);
  const [kbErrorMsg, setKbErrorMsg] = useState<string | null>(null);

  const [isSeededDraft, setIsSeededDraft] = useState<boolean>(initialContent !== undefined);
  const [docMode, setDocMode] = useState<DocWorkspaceMode>(initialMode ?? (initialContent !== undefined ? 'split' : 'read'));
  const [formatOverride, setFormatOverride] = useState<DocFormat>('auto');
  const [draft, setDraft] = useState<string>(initialContent ?? '');
  const [savedSnapshot, setSavedSnapshot] = useState<string>('');
  const [saving, setSaving] = useState<boolean>(false);
  const [saveFeedback, setSaveFeedback] = useState<string | null>(null);
  const editorRef = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (path.length > 0) setActivePath(path);
    if (initialContent !== undefined) {
      setIsSeededDraft(true);
      setDraft(initialContent);
      setSavedSnapshot('');
      setDocMode(initialMode ?? 'split');
      setSaveFeedback(null);
      setFormatOverride('auto');
    } else {
      setIsSeededDraft(false);
      if (initialMode) setDocMode(initialMode);
    }
  }, [path, initialContent, initialMode]);

  const effectivePath = activePath;

  const q = useVoxQuery(['doc', effectivePath], () => voxTransport.readDocMarkdown(effectivePath), {
    enabled: effectivePath.length > 0 && !isSeededDraft,
  });

  useEffect(() => {
    if (!isSeededDraft && typeof q.data === 'string') {
      setDraft(q.data);
      setSavedSnapshot(q.data);
      setSaveFeedback(null);
      setFormatOverride('auto');
    }
  }, [q.data, effectivePath, isSeededDraft]);

  const loadKbLibrary = useCallback(async (queryText: string) => {
    if (typeof voxTransport.invokeMcpTool !== 'function') return;
    try {
      const res = await voxTransport.invokeMcpTool('vox_kb_search', {
        query: queryText.trim() || 'architecture',
        limit: 12,
      });
      const data = unwrapMcpEnvelope((res as McpInvokeResult | undefined)?.result) as
        | { hits?: Array<Record<string, unknown>> }
        | undefined;
      if (data && Array.isArray(data.hits)) {
        setLibraryItems(mapKbSearchHits(data.hits));
        return;
      }
      if (!queryText.trim() && typeof voxTransport.voxDocsIndex === 'function') {
        const docs = await voxTransport.voxDocsIndex();
        if (Array.isArray(docs)) {
          setLibraryItems(
            docs.slice(0, 8).map((d) => ({
              path: d.path,
              title: d.title || d.path,
              topic: 'doc-governance',
              domain: 'documentation',
              trustTier: 'verified',
              rrfScore: 1,
            })),
          );
        }
      }
    } catch (err) {
      if (!(err instanceof BackendUnavailableError) && !(err instanceof Error && err.name === 'BackendUnavailableError')) {
        setKbErrorMsg(sanitizeErrorForToast(err));
      }
    }
  }, []);

  const loadInboxAndHealth = useCallback(async () => {
    if (typeof voxTransport.invokeMcpTool !== 'function') return;
    try {
      const [inboxRes, statusRes] = await Promise.all([
        voxTransport.invokeMcpTool('vox_kb_inbox_list', {}),
        voxTransport.invokeMcpTool('vox_kb_status', {}),
      ]);
      const inboxData = unwrapMcpEnvelope((inboxRes as McpInvokeResult | undefined)?.result) as
        | { findings?: Array<Record<string, unknown>> }
        | undefined;
      if (inboxData && Array.isArray(inboxData.findings)) {
        setInboxItems(mapInboxFindings(inboxData.findings));
      }
      const stData = unwrapMcpEnvelope((statusRes as McpInvokeResult | undefined)?.result);
      if (stData && typeof stData === 'object') setKbStatus(stData as KbStatusDto);
    } catch (err) {
      if (!(err instanceof BackendUnavailableError) && !(err instanceof Error && err.name === 'BackendUnavailableError')) {
        setKbErrorMsg(sanitizeErrorForToast(err));
      }
    }
  }, []);

  useEffect(() => {
    void loadKbLibrary(searchQuery);
  }, [searchQuery, loadKbLibrary]);

  useEffect(() => {
    void loadInboxAndHealth();
  }, [loadInboxAndHealth]);

  const activeContent = draft || q.data || '';
  const dirty = draft !== savedSnapshot;
  const resolvedFormat =
    formatOverride === 'auto' ? detectDocFormat(effectivePath, activeContent) : formatOverride;

  const provenance = useMemo(() => {
    const match = activeContent.match(PROVENANCE_REGEX);
    if (!match) return null;
    const sessionId = parseInt(match[1], 10);
    const stabilityScore = parseFloat(match[2]);
    return isNaN(sessionId) || isNaN(stabilityScore) ? null : { sessionId, stabilityScore };
  }, [activeContent]);

  const statusInfo = useMemo(() => resolveDocStatusInfo(activeContent), [activeContent]);
  const headings = useMemo(
    () => extractDocHeadings(activeContent, formatOverride, effectivePath),
    [activeContent, formatOverride, effectivePath],
  );

  const wordCount = useMemo(() => {
    const trimmed = activeContent.trim();
    return trimmed ? trimmed.split(/\s+/).length : 0;
  }, [activeContent]);
  const lineCount = useMemo(() => (activeContent ? activeContent.split(/\r?\n/).length : 0), [activeContent]);

  const filteredSearchResults = useMemo(() => {
    return libraryItems.filter((item) => {
      if (topicFilter !== 'all' && item.topic !== topicFilter) return false;
      if (domainFilter !== 'all' && item.domain !== domainFilter) return false;
      if (trustFilter !== 'all' && item.trustTier !== trustFilter) return false;
      if (
        searchQuery.trim().length > 0 &&
        !item.title.toLowerCase().includes(searchQuery.toLowerCase()) &&
        !item.path.toLowerCase().includes(searchQuery.toLowerCase())
      ) {
        return false;
      }
      return true;
    });
  }, [libraryItems, topicFilter, domainFilter, trustFilter, searchQuery]);

  const handleReviewFinding = async (id: string, decision: 'promote' | 'reject') => {
    setKbErrorMsg(null);
    setInboxItems((prev) =>
      prev.map((item) =>
        item.id === id
          ? { ...item, trustTier: decision === 'promote' ? 'verified_finding' : 'rejected' }
          : item,
      ),
    );
    if (typeof voxTransport.invokeMcpTool === 'function') {
      try {
        await voxTransport.invokeMcpTool('vox_kb_review_finding', {
          finding_id: id,
          action: decision,
          target_scope: promoteScope,
        });
      } catch (err) {
        setKbErrorMsg(sanitizeErrorForToast(err));
      }
    }
  };

  const handleSelectDoc = (docPath: string) => {
    setActivePath(docPath);
    onSelectDocPath?.(docPath);
    setActiveTab('reader');
  };

  const cycleFormat = () => {
    const cycle = listDocFormatCycle();
    setFormatOverride((prev) => cycle[(cycle.indexOf(prev) + 1) % cycle.length]!);
  };

  const insertSnippet = (before: string, after = '', fallbackBody = '') => {
    const el = editorRef.current;
    if (!el) {
      setDraft((prev) => `${prev}${prev.endsWith('\n') || !prev ? '' : '\n'}${before}${fallbackBody}${after}`);
      return;
    }
    const start = el.selectionStart ?? draft.length;
    const end = el.selectionEnd ?? start;
    const selected = draft.slice(start, end) || fallbackBody;
    const nextText = `${draft.slice(0, start)}${before}${selected}${after}${draft.slice(end)}`;
    setDraft(nextText);
    requestAnimationFrame(() => {
      el.focus();
      const cursor = start + before.length + selected.length;
      el.setSelectionRange(cursor, cursor);
    });
  };

  const handleSave = async () => {
    if (!effectivePath) return;
    setSaving(true);
    setKbErrorMsg(null);
    setSaveFeedback(null);
    try {
      if (typeof voxTransport.writeDocMarkdown === 'function') {
        await voxTransport.writeDocMarkdown(effectivePath, draft);
      }
      setSavedSnapshot(draft);
      setIsSeededDraft(false);
      setSaveFeedback('Saved');
    } catch (err) {
      setKbErrorMsg(sanitizeErrorForToast(err));
    } finally {
      setSaving(false);
    }
  };

  const handleNewDoc = () => {
    const stamp = new Date().toISOString().slice(0, 10);
    const nextPath = `docs/src/architecture/note-${stamp}-draft.md`;
    setIsSeededDraft(true);
    setActivePath(nextPath);
    setDocMode('split');
    setFormatOverride('auto');
    setDraft(NEW_DOC_TEMPLATE);
    setSavedSnapshot('');
    setSaveFeedback(null);
    setActiveTab('reader');
  };

  if (q.isLoading && !isSeededDraft) {
    return (
      <article className="p-6 font-display text-sm text-text-muted" data-testid="doc-reader">
        Loading documentation…
      </article>
    );
  }

  if (q.isError && !isSeededDraft) {
    return (
      <article className="p-6 font-mono text-sm text-(--color-status-warn)" data-testid="doc-reader" role="alert">
        Failed to load doc: {q.error.message}
      </article>
    );
  }

  const pendingInboxCount = inboxItems.filter(
    (i) => i.trustTier === 'candidate' || i.trustTier === 'quarantined_web',
  ).length;

  return (
    <article className="max-w-none p-2" data-testid="doc-reader">
      <div className="mb-3 flex flex-wrap items-center justify-between gap-2 border-b border-(--color-gold-500)/45 pb-2.5" data-testid="kb-library-tabs">
        <div role="tablist" aria-label="Knowledge Base Reader Sections" className="inline-flex max-w-full items-center gap-0.5 overflow-x-auto rounded-lg border border-border-strong bg-bg-surface p-1">
          {(
            [
              ['reader', 'Library & Reader'],
              ['inbox', `Findings Inbox (${pendingInboxCount})`],
              ['openwiki', 'OpenWiki Self-Heal'],
              ['health', 'KB Health & Freshness'],
            ] as const
          ).map(([tabKey, label]) => (
            <button
              key={tabKey}
              type="button"
              role="tab"
              data-slot="tab"
              aria-selected={activeTab === tabKey}
              data-testid={`kb-tab-${tabKey}`}
              onClick={() => setActiveTab(tabKey)}
              className={`inline-flex h-8 shrink-0 items-center whitespace-nowrap rounded px-2 text-[11px] tracking-normal transition-colors ${
                activeTab === tabKey
                  ? 'border border-(--color-gold-400) bg-bg-elevated text-text-primary font-semibold'
                  : 'border border-transparent font-medium text-text-secondary hover:text-text-primary'
              }`}
            >
              {label}
            </button>
          ))}
        </div>
      </div>

      {kbErrorMsg && (
        <div role="alert" data-testid="kb-reader-error" className="mb-3 rounded border border-(--color-status-error)/40 bg-(--color-status-error)/10 px-3 py-2 font-mono text-xs text-(--color-status-error)">
          {kbErrorMsg}
        </div>
      )}

      {activeTab === 'reader' && (
        <DocReaderLibraryBar
          libraryItems={libraryItems}
          filteredResults={filteredSearchResults}
          effectivePath={effectivePath}
          searchQuery={searchQuery}
          topicFilter={topicFilter}
          domainFilter={domainFilter}
          trustFilter={trustFilter}
          onSearchChange={setSearchQuery}
          onTopicChange={setTopicFilter}
          onDomainChange={setDomainFilter}
          onTrustChange={setTrustFilter}
          onSelectDoc={handleSelectDoc}
        />
      )}

      {activeTab === 'inbox' && (
        <DocReaderInboxPanel
          inboxItems={inboxItems}
          promoteScope={promoteScope}
          onScopeChange={setPromoteScope}
          onReview={(id, dec) => void handleReviewFinding(id, dec)}
        />
      )}

      {activeTab === 'openwiki' && (
        <div className="mb-4">
          <OpenWikiHealStudio onSelectDocPath={handleSelectDoc} />
        </div>
      )}

      {activeTab === 'health' && (
        <DocReaderHealthPanel
          kbStatus={kbStatus}
          staleVerifiedCount={staleVerifiedCount}
          onSelectDoc={handleSelectDoc}
          onVerifyStale={async () => {
            setStaleVerifiedCount((c) => c + 1);
            if (typeof voxTransport.invokeMcpTool === 'function') {
              try {
                await voxTransport.invokeMcpTool('vox_kb_verify', {});
              } catch (err) {
                setKbErrorMsg(sanitizeErrorForToast(err));
              }
            }
          }}
        />
      )}

      <header className="mb-3 flex flex-col gap-2 border-b border-(--color-gold-500)/45 pb-2.5 sm:flex-row sm:items-center sm:justify-between">
        <div className="min-w-0 flex-1">
          <h1 className="font-display text-lg font-semibold text-text-primary truncate">{effectivePath.split('/').pop()}</h1>
          {docMode !== 'read' ? (
            <input
              type="text"
              data-testid="doc-path-input"
              aria-label="Document workspace path"
              value={effectivePath}
              onChange={(e) => setActivePath(e.target.value)}
              className="mt-1 h-7 w-full max-w-md rounded border border-border-subtle bg-bg-elevated px-2 font-mono text-xs text-text-primary outline-hidden focus:border-brass"
            />
          ) : (
            <p className="mt-0.5 font-mono text-xs text-text-secondary truncate">{effectivePath}</p>
          )}
        </div>
        <div className="flex flex-wrap items-center gap-1.5">
          <span
            data-testid="kb-trust-tier-badge"
            className="inline-flex h-7 items-center rounded-full border border-(--color-status-pass)/50 bg-bg-surface px-2.5 font-mono text-[11px] font-semibold text-(--color-status-pass)"
          >
            {statusInfo?.trustBadge ?? 'Verified SSOT'}
          </span>
          {provenance && (
            <EmpiricalVerificationBadge
              sessionId={provenance.sessionId}
              stabilityScore={provenance.stabilityScore}
              onNavigateToResearch={onNavigateToResearch}
            />
          )}
        </div>
      </header>

      {/* Mode Switcher (Read | Split | Edit), Format Override & Quick-Insert Toolbar */}
      <div
        data-testid="doc-editor-toolbar"
        className="mb-3 flex flex-wrap items-center justify-between gap-2 rounded-lg border border-border-strong bg-bg-elevated p-2"
      >
        <div className="flex flex-wrap items-center gap-1.5">
          <div role="group" aria-label="Document workspace mode" className="inline-flex items-center gap-0.5 rounded border border-border-strong bg-bg-surface p-0.5">
            {(
              [
                ['read', 'Read'],
                ['split', 'Split'],
                ['edit', 'Edit'],
              ] as const
            ).map(([mKey, mLabel]) => (
              <button
                key={mKey}
                type="button"
                data-slot="button"
                data-testid={`doc-mode-${mKey}`}
                aria-pressed={docMode === mKey}
                onClick={() => setDocMode(mKey)}
                className={`inline-flex h-8 items-center rounded px-2 text-[11px] font-medium transition-colors ${
                  docMode === mKey
                    ? 'bg-brass/20 text-text-primary font-semibold'
                    : 'text-text-secondary hover:text-text-primary'
                }`}
              >
                {mLabel}
              </button>
            ))}
          </div>

          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            data-testid="doc-new-file-btn"
            title="Start a blank document with valid Vox frontmatter in Split editor mode"
            onClick={handleNewDoc}
            className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2.5 text-[11px] transition-colors"
          >
            <span>+ New Doc</span>
          </button>

          <button
            type="button"
            data-slot="button"
            data-roman-plaque="true"
            data-testid="doc-format-cycle-btn"
            title="Switch document parser format (Auto, Markdown, MDX, AsciiDoc, JSON, YAML, TOML, CSV, Code, Text)"
            onClick={cycleFormat}
            className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2.5 text-[11px] transition-colors"
          >
            <span>Format: {formatOverride === 'auto' ? `Auto (${resolvedFormat.toUpperCase()})` : resolvedFormat.toUpperCase()}</span>
          </button>

          {docMode !== 'read' && (
            <div className="flex flex-wrap items-center gap-1">
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('\n## ', '\n', 'Section')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + H2
              </button>
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('**', '**', 'bold')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + Bold
              </button>
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('\n```vox\n', '\n```\n', 'fn example() {}')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + Code
              </button>
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('\n| Column A | Column B |\n|---|---|\n| value | value |\n')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + Table
              </button>
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('\n> [!NOTE]\n> ', '\n', 'Important context')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + Callout
              </button>
              <button
                type="button"
                data-slot="button"
                onClick={() => insertSnippet('\n- [ ] ', '\n', 'Action item')}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2 font-mono text-[11px] text-text-secondary hover:text-text-primary"
              >
                + Task
              </button>
            </div>
          )}
        </div>

        <div className="flex flex-wrap items-center gap-1.5">
          <span className="font-mono text-[11px] text-text-muted">
            {lineCount}L · {wordCount}W{dirty ? ' · Unsaved' : saveFeedback ? ` · ${saveFeedback}` : ''}
          </span>
          {docMode !== 'read' && (
            <>
              <button
                type="button"
                data-slot="button"
                data-testid="doc-revert-btn"
                disabled={!dirty}
                onClick={() => {
                  setDraft(savedSnapshot);
                  setSaveFeedback(null);
                }}
                className="inline-flex h-8 items-center rounded border border-border-subtle bg-bg-surface px-2.5 text-[11px] text-text-secondary hover:text-text-primary disabled:opacity-45"
              >
                Revert
              </button>
              <button
                type="button"
                data-slot="button"
                data-roman-plaque="true"
                data-testid="doc-save-btn"
                disabled={saving || !dirty}
                onClick={() => void handleSave()}
                className="vox-brass-plaque-btn inline-flex h-8 items-center rounded px-2.5 text-[11px] disabled:opacity-45 transition-colors"
              >
                {saving ? 'Saving…' : 'Save'}
              </button>
            </>
          )}
        </div>
      </div>

      {headings.length > 1 && docMode !== 'edit' && (
        <nav
          aria-label="Document sections"
          data-testid="doc-toc-bar"
          className="mb-3 flex flex-wrap items-center gap-1 rounded-lg border border-border-subtle bg-bg-surface px-2.5 py-1.5"
        >
          <span className="mr-1 font-display text-[11px] font-semibold uppercase tracking-wider text-text-muted">
            Sections:
          </span>
          {headings.slice(0, 8).map((h) => (
            <button
              key={h.id}
              type="button"
              data-slot="button"
              onClick={() => {
                document.getElementById(h.id)?.scrollIntoView({ behavior: 'smooth', block: 'start' });
              }}
              className="inline-flex h-7 items-center rounded border border-border-subtle bg-bg-elevated px-2 font-sans text-[11px] text-text-secondary hover:border-brass/50 hover:text-text-primary transition-colors"
            >
              {h.text}
            </button>
          ))}
        </nav>
      )}

      {statusInfo && (
        <div data-testid="doc-status-banner" role="note" className={`mb-3 rounded-lg border p-3 text-xs leading-relaxed ${statusInfo.tone}`}>
          <strong>{statusInfo.label}:</strong> {statusInfo.description}
        </div>
      )}

      <div className={docMode === 'split' ? 'grid grid-cols-1 lg:grid-cols-2 gap-3 items-start' : ''}>
        {docMode !== 'read' && (
          <div className="flex flex-col gap-1.5">
            <label htmlFor="doc-editor-input" className="sr-only">
              Document source editor
            </label>
            <textarea
              id="doc-editor-input"
              ref={editorRef}
              data-testid="doc-editor-textarea"
              aria-label="Document source editor"
              value={draft}
              onChange={(e) => {
                setDraft(e.target.value);
                setSaveFeedback(null);
              }}
              rows={18}
              spellCheck={false}
              className="w-full min-h-[22rem] rounded-lg border border-border-strong bg-bg-elevated p-3 font-mono text-xs leading-relaxed text-text-primary outline-hidden focus:border-brass"
            />
          </div>
        )}

        {docMode !== 'edit' && (
          <div className="vox-marble-tablet rounded-lg border border-(--color-gold-500)/55 bg-bg-surface p-4 shadow-xs">
            <ResearchReportMarkdown
              markdown={activeContent}
              path={effectivePath}
              format={formatOverride}
              showFrontmatterCard
            />
          </div>
        )}
      </div>

      <div className="mt-3 flex flex-wrap items-center gap-2">
        <Button
          variant="outline"
          size="md"
          className="h-8 border-(--color-gold-500)/55 bg-bg-surface px-3 text-xs font-semibold text-text-primary"
          onClick={() => {
            voxTransport.openLocator({ kind: 'file', value: effectivePath }).catch((err) => {
              setKbErrorMsg(sanitizeErrorForToast(err));
            });
          }}
        >
          Open in external editor
        </Button>
      </div>
    </article>
  );
}
