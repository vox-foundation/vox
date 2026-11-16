export type ShellMode = 'default' | 'shell' | 'agent' | 'nu' | 'vox' | 'slash';

export type ResolvedIntentKind = 'shell' | 'nu' | 'vox' | 'agent' | 'slash';

export interface PromptIntentResult {
  kind: ResolvedIntentKind;
  modeBadge: ResolvedIntentKind;
  showAiAssistPill: boolean;
  toSubmitPayload: (raw: string) => string;
}

const SLASH_VERB_RE = /^\/[a-zA-Z][\w-]*(?:\s|$)/;
const ASK_SLASH_RE = /^\/(?:ask|ai)(?:\s|$)/i;
const VOX_SCRIPT_TOKEN_RE = /(?:^|\s)\S+\.vox(?:\s|$)/;
const NL_LEAD_RE = /^(?:how|why|what|where|when|explain|fix|summarize|list\s+all)\b/i;
const SHELL_OPERATOR_OR_FLAG_RE = /[|&;>$/]|(?:^|\s)--/;

function shouldShowAiAssistHint(trimmed: string, shellMode: ShellMode): boolean {
  if (shellMode !== 'default' && shellMode !== 'shell') return false;
  if (!trimmed || trimmed.includes('\n')) return false;
  const words = trimmed.split(/\s+/).filter(Boolean);
  if (words.length < 3) return false;
  if (!NL_LEAD_RE.test(trimmed)) return false;
  if (SHELL_OPERATOR_OR_FLAG_RE.test(trimmed)) return false;
  return true;
}

export function classifyPromptIntent(
  value: string,
  shellMode: ShellMode = 'default',
): PromptIntentResult {
  const trimmed = value.trim();

  // 1. Explicit agent prefix (`?...`, `/ask ...`, or `/ai ...`)
  if (trimmed.startsWith('?') || ASK_SLASH_RE.test(trimmed)) {
    return {
      kind: 'agent',
      modeBadge: 'agent',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => raw.trim(),
    };
  }

  // 2. Explicit slash command (`/verb ...`, excluding POSIX file paths like `/usr/bin/env`)
  if (SLASH_VERB_RE.test(trimmed)) {
    return {
      kind: 'slash',
      modeBadge: 'slash',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => raw.trim(),
    };
  }

  // 3. Explicit `!nu` structured Nushell pipeline
  if (trimmed === '!nu' || trimmed.startsWith('!nu ') || trimmed.startsWith('!nu\n')) {
    return {
      kind: 'nu',
      modeBadge: 'nu',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => raw.trim(),
    };
  }

  // 4. Explicit `!` shell override (strips only the single leading `!`)
  if (trimmed.startsWith('!')) {
    return {
      kind: 'shell',
      modeBadge: 'shell',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => raw.trim().slice(1).trimStart(),
    };
  }

  // 5. Locked mode routing
  if (shellMode === 'agent') {
    return {
      kind: 'agent',
      modeBadge: 'agent',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => {
        const t = raw.trim();
        if (!t) return '';
        return t.startsWith('?') || ASK_SLASH_RE.test(t) ? t : `?${t}`;
      },
    };
  }

  if (shellMode === 'nu') {
    return {
      kind: 'nu',
      modeBadge: 'nu',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => {
        const t = raw.trim();
        if (!t) return '';
        if (t.startsWith('/') || t.startsWith('?') || t === '!nu' || t.startsWith('!nu ')) {
          return t;
        }
        return `!nu ${t}`;
      },
    };
  }

  if (shellMode === 'slash') {
    return {
      kind: 'slash',
      modeBadge: 'slash',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => {
        const t = raw.trim();
        if (!t) return '';
        return t.startsWith('/') ? t : `/${t}`;
      },
    };
  }

  if (
    shellMode === 'vox' ||
    (shellMode === 'default' &&
      (trimmed === 'vox' || trimmed.startsWith('vox ') || VOX_SCRIPT_TOKEN_RE.test(trimmed)))
  ) {
    return {
      kind: 'vox',
      modeBadge: 'vox',
      showAiAssistPill: false,
      toSubmitPayload: (raw: string) => raw.trim(),
    };
  }

  // 6. Default / shell mode with non-hijacking AI Assist hint
  return {
    kind: 'shell',
    modeBadge: 'shell',
    showAiAssistPill: shouldShowAiAssistHint(trimmed, shellMode),
    toSubmitPayload: (raw: string) => raw.trim(),
  };
}

export function formatExecutionPayload(raw: string, shellMode: ShellMode = 'default'): string {
  return classifyPromptIntent(raw, shellMode).toSubmitPayload(raw);
}
