import React, { useState, useEffect, useMemo, useCallback } from 'react';
import {
  Dialog,
  DialogContent,
  DialogTitle,
  DialogDescription,
} from '../../ui/Dialog';
import { sanitizeErrorForToast } from '../../../lib/backendGuard';
import { ResearchReportMarkdown } from './ResearchReportMarkdown';
import {
  generateResearchDocDraft,
  publishResearchDoc,
  type DocDraftPreview,
  type PublishDocResult,
} from './researchActions';

export type { DocDraftPreview, PublishDocResult };


export interface DocPublishModalProps {
  sessionId: number;
  isOpen: boolean;
  onClose: () => void;
  onPublished: (relativePath: string) => void;
}

type TabType = 'preview' | 'raw' | 'validation';

interface ValidationCheck {
  id: string;
  label: string;
  valid: boolean;
}

export function sanitizeSlug(input: string): string {
  return input
    .toLowerCase()
    .replace(/[^a-z0-9-]/g, '-')
    .replace(/-+/g, '-');
}

function validateFrontmatter(md: string): ValidationCheck[] {
  const trimmed = md.trimStart();
  const fenceMatch = trimmed.match(/^---\r?\n([\s\S]*?)\r?\n---(?:\r?\n|$)/);
  const hasFences = Boolean(fenceMatch);
  const frontmatter = fenceMatch ? fenceMatch[1] : '';

  const titleMatch = frontmatter.match(/^title:\s*(.+)$/m);
  const rawTitle = titleMatch ? titleMatch[1].trim().replace(/^["']|["']$/g, '').trim() : '';
  const hasTitle = Boolean(rawTitle);

  const descMatch = frontmatter.match(/^description:\s*(.+)$/m);
  const rawDesc = descMatch ? descMatch[1].trim().replace(/^["']|["']$/g, '').trim() : '';
  const hasDesc = Boolean(rawDesc);

  const catMatch = frontmatter.match(/^category:\s*["']?([^"'\r\n]+)["']?/m);
  const hasCategory = Boolean(catMatch && catMatch[1].trim() === 'Architecture SSOTs');

  const statusMatch = frontmatter.match(/^status:\s*["']?([^"'\r\n]+)["']?/m);
  const hasStatus = Boolean(statusMatch && statusMatch[1].trim() === 'current');

  return [
    { id: 'fences', label: 'YAML frontmatter fences (---)', valid: hasFences },
    { id: 'title', label: 'Title defined', valid: hasTitle },
    { id: 'description', label: 'Description defined', valid: hasDesc },
    { id: 'category', label: 'Category is "Architecture SSOTs"', valid: hasCategory },
    { id: 'status', label: 'Status is "current"', valid: hasStatus },
  ];
}

export function DocPublishModal({
  sessionId,
  isOpen,
  onClose,
  onPublished,
}: DocPublishModalProps) {
  const [activeTab, setActiveTab] = useState<TabType>('preview');
  const [loading, setLoading] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);
  const [publishing, setPublishing] = useState(false);
  const [publishError, setPublishError] = useState<string | null>(null);

  const [slug, setSlug] = useState('');
  const [content, setContent] = useState('');
  const [originalDraft, setOriginalDraft] = useState<{ slug: string; content: string } | null>(null);

  const loadDraft = useCallback(async () => {
    setLoading(true);
    setLoadError(null);
    try {
      const draft = await generateResearchDocDraft(sessionId);
      const initialSlug = sanitizeSlug(draft.slug);
      setSlug(initialSlug);
      setContent(draft.markdown_content);
      setOriginalDraft({ slug: initialSlug, content: draft.markdown_content });
    } catch (err) {
      setLoadError(sanitizeErrorForToast(err));
    } finally {
      setLoading(false);
    }
  }, [sessionId]);

  useEffect(() => {
    if (isOpen) {
      setPublishError(null);
      setActiveTab('preview');
      void loadDraft();
    }
  }, [isOpen, loadDraft]);

  const checks = useMemo(() => validateFrontmatter(content), [content]);
  const allChecksPass = useMemo(() => checks.every((c) => c.valid), [checks]);

  const handleReset = () => {
    if (originalDraft) {
      setSlug(originalDraft.slug);
      setContent(originalDraft.content);
    }
  };

  const cleanSlug = slug.trim().replace(/^-+|-+$/g, '');

  const handlePublish = async () => {
    const finalSlug = cleanSlug || 'unnamed';
    setPublishing(true);
    setPublishError(null);
    try {
      const res = await publishResearchDoc(sessionId, finalSlug, content);
      onPublished(res.relative_path);
      onClose();
    } catch (err) {
      setPublishError(sanitizeErrorForToast(err));
    } finally {
      setPublishing(false);
    }
  };


  const targetFilename = cleanSlug.endsWith('.md')
    ? cleanSlug
    : `${cleanSlug || 'unnamed'}-2026.md`;

  if (!isOpen) return null;

  return (
    <Dialog open={isOpen} onOpenChange={(open) => { if (!open) onClose(); }}>
      <DialogContent className="max-w-3xl max-h-[90vh] flex flex-col p-6 bg-[#121216] border border-border-subtle rounded-xl text-text-primary">
        <div className="flex items-start justify-between pb-3 border-b border-border-subtle">
          <div>
            <DialogTitle className="font-display text-base font-semibold text-text-primary tracking-wide">
              Publish Architecture SSOT
            </DialogTitle>
            <DialogDescription className="mt-0.5 text-xs text-text-muted">
              Review, edit, and publish the research findings to docs/src/architecture/.
            </DialogDescription>
          </div>
          <button
            type="button"
            onClick={onClose}
            aria-label="Close"
            className="text-text-muted hover:text-text-secondary text-lg leading-none px-2 py-1 rounded"
          >
            ✕
          </button>
        </div>

        {loadError && (
          <div className="mt-3 p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-xs text-red-400">
            Failed to generate draft: {loadError}
          </div>
        )}

        {publishError && (
          <div className="mt-3 p-3 rounded-lg border border-red-500/30 bg-red-500/10 text-xs text-red-400">
            Failed to publish document: {publishError}
          </div>
        )}

        {loading ? (
          <div className="py-16 text-center text-sm text-text-muted animate-pulse">
            Generating Architecture SSOT draft from session {sessionId}…
          </div>
        ) : (
          <div className="flex flex-col flex-1 min-h-0 space-y-4 pt-4">
            {/* Slug & Filename configuration */}
            <div className="flex flex-col sm:flex-row gap-3 items-start sm:items-center justify-between bg-overlay-subtle border border-border-subtle p-3 rounded-lg">
              <div className="flex-1 w-full flex items-center gap-2">
                <label htmlFor="doc-slug-input" className="text-xs text-text-muted font-medium shrink-0">
                  Slug:
                </label>
                <input
                  id="doc-slug-input"
                  type="text"
                  value={slug}
                  onChange={(e) => setSlug(sanitizeSlug(e.target.value))}
                  placeholder="document-slug"
                  className="flex-1 rounded border border-border-subtle bg-black/40 px-2.5 py-1.5 text-xs text-text-secondary outline-none focus:border-brass/50 font-mono"
                />
              </div>
              <div className="text-[11px] font-mono text-text-muted shrink-0">
                Target: <span className="text-brass/90">docs/src/architecture/{targetFilename}</span>
              </div>
            </div>

            {/* Tab Navigation */}
            <div className="flex items-center gap-2 border-b border-border-subtle pb-2">
              <button
                type="button"
                onClick={() => setActiveTab('preview')}
                className={`px-3 py-1.5 text-xs rounded transition-colors ${
                  activeTab === 'preview'
                    ? 'border border-brass/40 bg-brass/15 text-brass font-medium'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                Preview
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('raw')}
                className={`px-3 py-1.5 text-xs rounded transition-colors ${
                  activeTab === 'raw'
                    ? 'border border-brass/40 bg-brass/15 text-brass font-medium'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                Raw Markdown
              </button>
              <button
                type="button"
                onClick={() => setActiveTab('validation')}
                className={`px-3 py-1.5 text-xs rounded transition-colors flex items-center gap-1.5 ${
                  activeTab === 'validation'
                    ? 'border border-brass/40 bg-brass/15 text-brass font-medium'
                    : 'text-text-muted hover:text-text-secondary'
                }`}
              >
                <span>Validation</span>
                <span
                  className={`inline-block w-2 h-2 rounded-full ${
                    allChecksPass ? 'bg-emerald-400' : 'bg-amber-400'
                  }`}
                />
              </button>
            </div>

            {/* Tab Contents */}
            <div className="flex-1 min-h-[300px] max-h-[460px] overflow-y-auto">
              {activeTab === 'preview' && (
                <div className="p-4 bg-black/20 rounded-lg border border-border-subtle overflow-y-auto">
                  <ResearchReportMarkdown markdown={content || '(No markdown content)'} />
                </div>
              )}

              {activeTab === 'raw' && (
                <div className="h-full flex flex-col">
                  <textarea
                    aria-label="Raw Markdown Content"
                    value={content}
                    onChange={(e) => setContent(e.target.value)}
                    rows={18}
                    className="w-full h-full min-h-[320px] font-mono text-xs p-3 rounded-lg border border-border-subtle bg-black/40 text-text-secondary outline-none focus:border-brass/40 resize-none leading-relaxed"
                  />
                </div>
              )}

              {activeTab === 'validation' && (
                <div className="p-4 bg-black/20 rounded-lg border border-border-subtle space-y-4">
                  <div className="flex items-center justify-between pb-2 border-b border-border-subtle">
                    <h3 className="text-xs font-semibold uppercase tracking-wider text-text-muted">
                      Frontmatter Validation
                    </h3>
                    <span
                      className={`text-[11px] px-2 py-0.5 rounded font-mono ${
                        allChecksPass
                          ? 'text-emerald-400 bg-emerald-500/10 border border-emerald-500/30'
                          : 'text-amber-400 bg-amber-500/10 border border-amber-500/30'
                      }`}
                    >
                      {allChecksPass ? 'All Checks Passed' : 'Checks Failing'}
                    </span>
                  </div>

                  <ul className="space-y-2">
                    {checks.map((c) => (
                      <li
                        key={c.id}
                        className="flex items-center justify-between p-2.5 rounded bg-overlay-subtle border border-border-subtle text-xs"
                      >
                        <span className="text-text-secondary">{c.label}</span>
                        {c.valid ? (
                          <span className="text-emerald-400 font-mono flex items-center gap-1">
                            ✓ Passed
                          </span>
                        ) : (
                          <span className="text-red-400 font-mono flex items-center gap-1">
                            ✕ Missing or Invalid
                          </span>
                        )}
                      </li>
                    ))}
                  </ul>

                  {!allChecksPass && (
                    <div className="text-[11px] text-amber-400/90 bg-amber-500/10 border border-amber-500/20 p-2.5 rounded">
                      Ensure the document starts with YAML frontmatter containing <code>title</code>,{' '}
                      <code>description</code>, <code>category: &quot;Architecture SSOTs&quot;</code>, and{' '}
                      <code>status: &quot;current&quot;</code>.
                    </div>
                  )}
                </div>
              )}
            </div>

            {/* Modal Actions */}
            <div className="flex items-center justify-between pt-3 border-t border-border-subtle mt-2">
              <button
                type="button"
                onClick={handleReset}
                disabled={!originalDraft || publishing}
                className="rounded border border-border-subtle px-3 py-1.5 text-xs text-text-muted hover:text-text-secondary hover:border-border-base transition-colors disabled:opacity-50"
              >
                Reset
              </button>

              <div className="flex items-center gap-2">
                <button
                  type="button"
                  onClick={onClose}
                  disabled={publishing}
                  className="rounded border border-border-subtle px-3 py-1.5 text-xs text-text-muted hover:text-text-secondary hover:border-border-base transition-colors disabled:opacity-50"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handlePublish}
                  disabled={publishing || !slug.trim() || loading}
                  className="rounded border border-brass/40 bg-brass/20 hover:bg-brass/30 text-brass px-4 py-1.5 text-xs font-medium transition-colors disabled:opacity-50 flex items-center gap-1.5"
                >
                  {publishing ? 'Publishing…' : 'Approve & Publish'}
                </button>
              </div>
            </div>
          </div>
        )}
      </DialogContent>
    </Dialog>
  );
}
