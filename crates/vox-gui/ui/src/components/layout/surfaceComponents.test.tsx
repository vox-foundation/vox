// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import React from 'react';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';

vi.mock('../../transport', async (importOriginal) => {
  const actual = await importOriginal<typeof import('../../transport')>();
  return {
    ...actual,
    listenSecretaryProposed: vi.fn(() => Promise.resolve(() => {})),
  };
});

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn().mockResolvedValue(null),
}));

import { LanguageProvider } from '../../hooks/useLanguage';
import { INITIAL_DATA } from '../../data/initialState';
import type { AttentionInbox } from '../../hooks/useAttentionInbox';
import { renderSurfaceContent, type SurfaceProps } from './surfaceComponents';

const attention: AttentionInbox = {
  approvals: [
    { approval_id: 'a1', tool: 'shell', summary: 'run tests', requested_at_ms: 1 },
    { approval_id: 'a2', tool: 'write', summary: 'edit file', requested_at_ms: 2 },
  ],
  needsYou: [],
  withheld: [],
  blockedTasksCount: 0,
  hopperTasks: [],
  totalCount: 2,
  refresh: vi.fn().mockResolvedValue(undefined),
  resolveApproval: vi.fn().mockResolvedValue(undefined),
  resolveFeedback: vi.fn().mockResolvedValue(undefined),
};

const stubProps: SurfaceProps = {
  pushToast: vi.fn(),
  data: INITIAL_DATA,
  chatComposer: <div>composer</div>,
  attention,
  onOpenFeedbackContext: vi.fn(),
  onNavigate: vi.fn(),
};

describe('renderSurfaceContent chat attention wiring', () => {
  beforeEach(() => {
    window.localStorage.clear();
  });

  it('forwards attention.approvals.length as the Approvals dock pending count', () => {
    const client = new QueryClient({ defaultOptions: { queries: { retry: false } } });
    render(
      <LanguageProvider>
        <QueryClientProvider client={client}>
          {renderSurfaceContent('chat', stubProps)}
        </QueryClientProvider>
      </LanguageProvider>,
    );
    fireEvent.click(screen.getByRole('button', { name: /panels/i }));
    fireEvent.click(screen.getByRole('checkbox', { name: /^approvals$/i }));
    expect(screen.getByTestId('chat-dock-approvals')).toHaveTextContent('2 pending');
  });
});
