import { LEAK_PATTERN } from '../../lib/backendGuard';
import type { InvariantViolation } from '../types';

const SYSTEM_CHROME_SELECTOR =
  '[data-testid="toast-item"], [role="alert"], [role="status"], header [data-testid="header-error"], [data-testid="error-boundary"]';
const EXCLUDE_CONTENT_SELECTOR =
  '.prose, .markdown-body, pre, code, [data-testid="chat-transcript"], [data-testid="terminal-stream"], [data-testid="inspector-drawer"]';

export function checkErrorLeakInvariant(
  root: Element | null = typeof document !== 'undefined' ? document.body : null
): InvariantViolation[] {
  if (!root) return [];

  const violations: InvariantViolation[] = [];
  const systemElements: HTMLElement[] = [];

  if (root instanceof HTMLElement && root.matches(SYSTEM_CHROME_SELECTOR)) {
    systemElements.push(root);
  }
  systemElements.push(...Array.from(root.querySelectorAll<HTMLElement>(SYSTEM_CHROME_SELECTOR)));

  for (const el of systemElements) {
    if (el.closest(EXCLUDE_CONTENT_SELECTOR)) continue;
    const text = el.textContent ?? '';

    if (
      LEAK_PATTERN.test(text) ||
      /\[object Object\]/.test(text) ||
      /undefined is not a/.test(text) ||
      /TypeError:/.test(text)
    ) {
      const elementSelector =
        el.getAttribute('data-testid') ? `[data-testid="${el.getAttribute('data-testid')}"]` :
        el.getAttribute('role') ? `${el.tagName.toLowerCase()}[role="${el.getAttribute('role')}"]` :
        el.tagName.toLowerCase();

      violations.push({
        kind: 'raw_error_leak',
        severity: 'critical',
        message: `Raw runtime exception leaked into system chrome: "${text.trim().slice(0, 80)}"`,
        elementSelector,
      });
    }
  }
  return violations;
}
