export type PromptTokenKind =
  | 'command'
  | 'subcommand'
  | 'flag'
  | 'string'
  | 'variable'
  | 'operator'
  | 'macro'
  | 'slash'
  | 'comment'
  | 'text';

export interface PromptToken {
  text: string;
  kind: PromptTokenKind;
}

export const PROMPT_TOKEN_CLASSES: Record<PromptTokenKind, string> = {
  command: 'text-accent font-normal',
  subcommand: 'text-text-primary font-normal',
  flag: 'text-(--color-accent-default) font-normal',
  string: 'text-(--color-status-pass) font-normal',
  variable: 'text-(--color-status-warn) font-normal',
  operator: 'text-text-secondary font-normal',
  macro: 'text-accent font-normal',
  slash: 'text-accent font-normal',
  comment: 'text-text-muted font-normal',
  text: 'text-text-primary font-normal',
};

const SUBCOMMAND_HOSTS = new Set([
  'vox', 'git', 'cargo', 'pnpm', 'npm', 'docker', 'kubectl', 'rustup', 'gh',
]);

function pushToken(tokens: PromptToken[], text: string, kind: PromptTokenKind): void {
  if (!text) return;
  const prev = tokens[tokens.length - 1];
  if (prev && prev.kind === kind) prev.text += text;
  else tokens.push({ text, kind });
}

export function tokenizePromptBuffer(value: string): PromptToken[] {
  if (!value) return [];
  const tokens: PromptToken[] = [];
  const len = value.length;
  let i = 0;
  let expectingCommand = true;
  let headCommand: string | null = null;
  let seenSubcommand = false;

  while (i < len) {
    const ch = value[i];
    if (ch === ' ' || ch === '\t' || ch === '\r' || ch === '\n') {
      let end = i + 1;
      let sawNewline = ch === '\n';
      while (end < len && /[\s]/.test(value[end])) {
        if (value[end] === '\n') sawNewline = true;
        end++;
      }
      pushToken(tokens, value.slice(i, end), 'text');
      if (sawNewline) {
        expectingCommand = true;
        headCommand = null;
        seenSubcommand = false;
      }
      i = end;
      continue;
    }

    if (ch === '#' && (i === 0 || /\s/.test(value[i - 1]))) {
      let end = i + 1;
      while (end < len && value[end] !== '\n') end++;
      pushToken(tokens, value.slice(i, end), 'comment');
      i = end;
      continue;
    }

    if (ch === "'" || ch === '"') {
      let end = i + 1;
      while (end < len) {
        if (value[end] === '\\' && end + 1 < len) {
          end += 2;
          continue;
        }
        if (value[end] === ch) {
          end++;
          break;
        }
        end++;
      }
      pushToken(tokens, value.slice(i, end), 'string');
      expectingCommand = false;
      i = end;
      continue;
    }

    const two = value.slice(i, i + 2);
    if (two === '&&' || two === '||' || two === '>>' || two === '<<') {
      pushToken(tokens, two, 'operator');
      if (two === '&&' || two === '||') {
        expectingCommand = true;
        headCommand = null;
        seenSubcommand = false;
      }
      i += 2;
      continue;
    }
    if (ch === '|' || ch === ';' || ch === '>' || ch === '<') {
      pushToken(tokens, ch, 'operator');
      if (ch === '|' || ch === ';') {
        expectingCommand = true;
        headCommand = null;
        seenSubcommand = false;
      }
      i++;
      continue;
    }

    if (ch === '$' && i + 1 < len && /[A-Za-z_{0-9?#@*!$]/.test(value[i + 1])) {
      let end = i + 1;
      if (value[end] === '{') {
        end++;
        while (end < len && value[end] !== '}' && value[end] !== '\n') end++;
        if (end < len && value[end] === '}') end++;
      } else {
        while (end < len && /[A-Za-z0-9_]/.test(value[end])) end++;
      }
      pushToken(tokens, value.slice(i, end), 'variable');
      expectingCommand = false;
      i = end;
      continue;
    }

    let end = i;
    while (end < len && !/[\s'"|&;><]/.test(value[end]) && !(value[end] === '$' && end > i)) {
      end++;
    }
    if (end === i) {
      pushToken(tokens, value[i], 'operator');
      i++;
      continue;
    }

    const word = value.slice(i, end);
    if (word.startsWith('@') && word.length > 1) {
      pushToken(tokens, word, 'macro');
      expectingCommand = false;
    } else if (expectingCommand && /^\/[A-Za-z][A-Za-z0-9_-]*$/.test(word)) {
      pushToken(tokens, word, 'slash');
      expectingCommand = false;
    } else if (expectingCommand) {
      pushToken(tokens, word, 'command');
      if (word !== '!nu') {
        expectingCommand = false;
        headCommand = word.replace(/^[!?]+/, '');
        seenSubcommand = false;
      }
    } else if (word.startsWith('-') && word.length >= 2) {
      pushToken(tokens, word, 'flag');
    } else if (headCommand && SUBCOMMAND_HOSTS.has(headCommand) && !seenSubcommand) {
      pushToken(tokens, word, 'subcommand');
      seenSubcommand = true;
    } else {
      pushToken(tokens, word, 'text');
    }
    i = end;
  }

  return tokens;
}

export const tokenizePromptLine = tokenizePromptBuffer;

function endsWithUnescapedBackslash(line: string): boolean {
  const trimmedRight = line.replace(/[ \t\r]+$/, '');
  let count = 0;
  for (let idx = trimmedRight.length - 1; idx >= 0 && trimmedRight[idx] === '\\'; idx--) count++;
  return count % 2 === 1;
}

export function isIncompleteContinuation(value: string): boolean {
  if (!value) return false;
  let inSingle = false;
  let inDouble = false;
  let escaped = false;

  for (let i = 0; i < value.length; i++) {
    const ch = value[i];
    if (escaped) {
      escaped = false;
      continue;
    }
    if (ch === '\\') {
      escaped = true;
      continue;
    }
    if (!inSingle && !inDouble && ch === '#' && (i === 0 || /\s/.test(value[i - 1]))) {
      while (i < value.length && value[i] !== '\n') i++;
      continue;
    }
    if (ch === "'" && !inDouble) inSingle = !inSingle;
    else if (ch === '"' && !inSingle) inDouble = !inDouble;
  }

  if (inSingle || inDouble) return true;
  const lines = value.split('\n');
  let lastNonEmpty = '';
  for (let idx = lines.length - 1; idx >= 0; idx--) {
    const stripped = lines[idx].replace(/(?:^|\s)#.*$/, '').trim();
    if (stripped.length > 0) {
      lastNonEmpty = stripped;
      break;
    }
  }
  if (!lastNonEmpty) return false;
  if (endsWithUnescapedBackslash(lines[lines.length - 1])) return true;
  return (
    lastNonEmpty.endsWith('&&') || lastNonEmpty.endsWith('||') || lastNonEmpty.endsWith('|')
  );
}
