/** Design tokens — status colors and badge class maps. */

export const STATUS_BADGE_CLASS = {
  pass: 'bg-(--color-status-pass)/20 text-(--color-status-pass) ring-1 ring-(--color-status-pass)/40',
  fail: 'bg-(--color-status-fail)/20 text-(--color-status-fail) ring-1 ring-(--color-status-fail)/40',
  warn: 'bg-(--color-status-warn)/20 text-(--color-status-warn) ring-1 ring-(--color-status-warn)/40',
  not_run: 'bg-white/5 text-zinc-400',
} as const;

export const STATUS_RAIL_BADGE_CLASS = {
  pass: 'bg-(--color-status-pass) text-bg-base',
  fail: 'bg-(--color-status-fail) text-bg-base',
  warn: 'bg-(--color-status-warn) text-bg-base',
  not_run: 'bg-zinc-600 text-zinc-100',
} as const;

export type StatusToneKind =
  | 'pass'
  | 'fail'
  | 'warn'
  | 'info'
  | 'neutral'
  | 'accent'
  | 'Executing'
  | 'Verifying'
  | 'Planning'
  | 'Paused'
  | 'Validated'
  | 'Doubted'
  | 'Speculative'
  | 'Active'
  | 'Root';

export const STATUS_TONE = {
  pass:   { dot: 'bg-(--color-status-pass)',  ring: 'ring-(--color-status-pass)/30',  text: 'text-(--color-status-pass)',  soft: 'bg-(--color-status-pass)/10',  solid: 'bg-(--color-status-pass)',  onSolid: 'text-bg-base' },
  fail:   { dot: 'bg-(--color-status-fail)',      ring: 'ring-(--color-status-fail)/30',      text: 'text-(--color-status-fail)',      soft: 'bg-(--color-status-fail)/10',      solid: 'bg-(--color-status-fail)',      onSolid: 'text-bg-base' },
  warn:   { dot: 'bg-(--color-status-warn)',    ring: 'ring-(--color-status-warn)/30',    text: 'text-(--color-status-warn)',    soft: 'bg-(--color-status-warn)/10',    solid: 'bg-(--color-status-warn)',    onSolid: 'text-bg-base' },
  // `info` is intentionally left as sky-400: it is a distinct, deliberately
  // used blue for neutral informational banners/toasts elsewhere in the app
  // (not one of the PhaseKind states reported here) and is out of scope for
  // this fix.
  info:   { dot: 'bg-sky-400',      ring: 'ring-sky-400/30',      text: 'text-sky-300',      soft: 'bg-sky-400/10',      solid: 'bg-sky-400',      onSolid: 'text-zinc-950' },
  neutral:{ dot: 'bg-zinc-500',     ring: 'ring-zinc-500/30',     text: 'text-zinc-300',     soft: 'bg-white/4',    solid: 'bg-zinc-500',     onSolid: 'text-zinc-100' },
  accent: { dot: 'bg-brass',        ring: 'ring-brass/30',        text: 'text-brass',        soft: 'bg-brass/10',        solid: 'bg-brass',        onSolid: 'text-zinc-950' },
  Executing:   { dot: 'bg-brass',     ring: 'ring-brass/30',       text: 'text-brass',       soft: 'bg-brass/10',       solid: 'bg-brass',       onSolid: 'text-zinc-950' },
  Verifying:   { dot: 'bg-violet-400',ring: 'ring-violet-400/30', text: 'text-violet-300',  soft: 'bg-violet-400/10',  solid: 'bg-violet-400',  onSolid: 'text-zinc-950' },
  // Planning/Active used to render as cool cyan-400 (a "blue" chip) — the
  // only two PhaseKind tones that broke from the app's brass/gold accent
  // language everywhere else (Approvals, Discovery, Repository never use a
  // blue accent). Both now reuse the existing `brass` token, same as
  // Executing — matching the flatter, single-accent convention those
  // "correctly themed" panels already follow instead of inventing a new hue.
  Planning:    { dot: 'bg-brass',     ring: 'ring-brass/30',       text: 'text-brass',       soft: 'bg-brass/10',       solid: 'bg-brass',       onSolid: 'text-zinc-950' },
  Paused:      { dot: 'bg-zinc-500',  ring: 'ring-zinc-500/30',    text: 'text-zinc-300',    soft: 'bg-white/4',   solid: 'bg-zinc-500',    onSolid: 'text-zinc-100' },
  Validated:   { dot: 'bg-(--color-status-pass)',ring:'ring-(--color-status-pass)/30', text: 'text-(--color-status-pass)', soft: 'bg-(--color-status-pass)/10', solid: 'bg-(--color-status-pass)', onSolid: 'text-bg-base' },
  Doubted:     { dot: 'bg-(--color-status-warn)', ring: 'ring-(--color-status-warn)/30',   text: 'text-(--color-status-warn)',   soft: 'bg-(--color-status-warn)/10',   solid: 'bg-(--color-status-warn)',   onSolid: 'text-bg-base' },
  Speculative: { dot: 'bg-violet-400',ring: 'ring-violet-400/30',  text: 'text-violet-300',  soft: 'bg-violet-400/10',  solid: 'bg-violet-400',  onSolid: 'text-zinc-950' },
  Active:      { dot: 'bg-brass',     ring: 'ring-brass/30',       text: 'text-brass',       soft: 'bg-brass/10',       solid: 'bg-brass',       onSolid: 'text-zinc-950' },
  Root:        { dot: 'bg-white',     ring: 'ring-white/30',       text: 'text-white',       soft: 'bg-white/6',   solid: 'bg-white',       onSolid: 'text-zinc-950' },
} as const;

