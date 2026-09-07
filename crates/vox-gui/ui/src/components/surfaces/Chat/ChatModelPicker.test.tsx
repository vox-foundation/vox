// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from 'vitest';
import { render, screen, fireEvent } from '@testing-library/react';
import userEvent from '@testing-library/user-event';
import React from 'react';
import { readFileSync } from 'node:fs';
import path from 'node:path';

const invoke = vi.fn();
vi.mock('@tauri-apps/api/core', () => ({ invoke: (...a: unknown[]) => invoke(...a) }));

import { ChatModelPicker } from './ChatModelPicker';

describe('ChatModelPicker', () => {
  beforeEach(() => {
    invoke.mockReset();
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_model_cards') {
        return [
          { id: 'openai/gpt-5.2-mini', provider: 'openai' },
          { id: 'anthropic/claude-opus-4.7', provider: 'anthropic' },
        ];
      }
      if (cmd === 'inference_provider_status') {
        return [
          { provider: 'OpenAI', key_present: true, is_local: false, local_reachable: null },
          { provider: 'Anthropic', key_present: true, is_local: false, local_reachable: null },
        ];
      }
      return null;
    });
  });

  it('loads the catalog on open and reports a pick via onApplied — never set_active_model', async () => {
    const user = userEvent.setup();
    const onApplied = vi.fn();
    render(<ChatModelPicker activeModel="openai/gpt-5.2-mini" onApplied={onApplied} />);
    await user.click(screen.getByRole('button', { name: /model: openai\/gpt-5\.2-mini/i }));
    await user.click(await screen.findByRole('option', { name: 'anthropic/claude-opus-4.7' }));
    expect(onApplied).toHaveBeenCalledWith('anthropic/claude-opus-4.7');
    // Honest wiring: set_active_model only touches the GUI process and is never
    // read by the daemon serving chat — the pick must NOT ride it.
    expect(invoke).not.toHaveBeenCalledWith('set_active_model', expect.anything());
  });

  it('offers auto-route to clear the override', async () => {
    const user = userEvent.setup();
    const onApplied = vi.fn();
    render(<ChatModelPicker activeModel="anthropic/claude-opus-4.7" onApplied={onApplied} />);
    await user.click(screen.getByRole('button', { name: /model: anthropic/i }));
    await user.click(await screen.findByRole('option', { name: /auto-route/i }));
    expect(onApplied).toHaveBeenCalledWith(null);
  });

  it('opens the listbox upward (bottom-full) so it clears the bottom-docked composer', async () => {
    const user = userEvent.setup();
    render(<ChatModelPicker activeModel={null} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    const listbox = await screen.findByRole('listbox', { name: /pick model/i });
    expect(listbox.className).toContain('bottom-full');
    expect(listbox.className).not.toContain('mt-1');
  });

  it('closes the listbox on Escape', async () => {
    const user = userEvent.setup();
    render(<ChatModelPicker activeModel={null} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    await screen.findByRole('listbox', { name: /pick model/i });
    await user.keyboard('{Escape}');
    expect(screen.queryByRole('listbox', { name: /pick model/i })).toBeNull();
  });

  it('closes the listbox on outside pointerdown', async () => {
    const user = userEvent.setup();
    render(<ChatModelPicker activeModel={null} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    await screen.findByRole('listbox', { name: /pick model/i });
    fireEvent.pointerDown(document.body);
    expect(screen.queryByRole('listbox', { name: /pick model/i })).toBeNull();
  });

  it('hides a model whose provider has no key configured', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_model_cards') {
        return [
          { id: 'openai/gpt-5.2-mini', provider: 'openai', provider_type: 'OpenAI' },
          { id: 'anthropic/claude-opus-4.7', provider: 'anthropic', provider_type: 'Anthropic' },
        ];
      }
      if (cmd === 'inference_provider_status') {
        return [
          { provider: 'OpenAI', key_present: false, is_local: false, local_reachable: null },
          { provider: 'Anthropic', key_present: true, is_local: false, local_reachable: null },
        ];
      }
      return null;
    });
    const user = userEvent.setup();
    const onApplied = vi.fn();
    render(<ChatModelPicker activeModel={null} onApplied={onApplied} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    expect(await screen.findByRole('option', { name: 'anthropic/claude-opus-4.7' })).toBeDefined();
    expect(screen.queryByRole('option', { name: /openai\/gpt-5\.2-mini/i })).toBeNull();
    expect(onApplied).not.toHaveBeenCalled();
  });

  it('hides a local provider model when the cached health probe reports unreachable', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_model_cards') return [{ id: 'ollama/llama3', provider: 'ollama' }];
      if (cmd === 'inference_provider_status') {
        return [{ provider: 'Ollama', key_present: true, is_local: true, local_reachable: false }];
      }
      return null;
    });
    const user = userEvent.setup();
    const onApplied = vi.fn();
    render(<ChatModelPicker activeModel={null} onApplied={onApplied} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    await screen.findByRole('searchbox', { name: /search models/i });
    expect(screen.queryByRole('option', { name: /ollama\/llama3/i })).toBeNull();
    expect(onApplied).not.toHaveBeenCalled();
  });

  it('narrows the keyed catalog through the search box, including OpenRouter ids', async () => {
    invoke.mockImplementation(async (cmd: string) => {
      if (cmd === 'list_model_cards') {
        return [
          { id: 'openrouter/anthropic/claude-sonnet-4', provider: 'anthropic', provider_type: 'OpenRouter' },
          { id: 'mens/e2e-smoke-metal', provider: 'populi_local', provider_type: 'VoxLocal' },
        ];
      }
      if (cmd === 'inference_provider_status') {
        return [
          { provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null },
          { provider: 'VoxLocal', key_present: true, is_local: true, local_reachable: null },
        ];
      }
      return null;
    });
    const user = userEvent.setup();
    render(<ChatModelPicker activeModel={null} />);
    await user.click(screen.getByRole('button', { name: /model: auto-route/i }));
    expect(await screen.findByRole('option', { name: 'openrouter/anthropic/claude-sonnet-4' })).toBeDefined();
    expect(screen.getByRole('option', { name: 'mens/e2e-smoke-metal' })).toBeDefined();
    await user.type(screen.getByRole('searchbox', { name: /search models/i }), 'openrouter');
    expect(screen.getByRole('option', { name: 'openrouter/anthropic/claude-sonnet-4' })).toBeDefined();
    expect(screen.queryByRole('option', { name: 'mens/e2e-smoke-metal' })).toBeNull();
  });
});

// Wiring guard: App mounts GroundingCheckToggle only in Loquela trailingSlot.
// The catalog picker is Loquela's "Choose model tier" control — ChatModelPicker
// stays as a module but is not mounted in the composer toolbar.
describe('ChatModelPicker toolbar placement wiring', () => {
  it('App.tsx does not mount ChatModelPicker in Loquela trailingSlot', () => {
    const appSrc = readFileSync(path.resolve(__dirname, '../../../App.tsx'), 'utf8');
    const loquelaBlockMatch = appSrc.match(/const loquelaComposer = \(\s*<Loquela[\s\S]*?\/>\s*\);/);
    expect(loquelaBlockMatch?.[0]).not.toMatch(/<ChatModelPicker/);
    expect(loquelaBlockMatch?.[0]).toMatch(/<GroundingCheckToggle/);
  });
});

// Wiring guard (readFileSync idiom, mirroring Phase 1's ErrorBoundary.test.tsx):
// the picked model must reach the submit payload App sends to the daemon.
describe('model_override submit-payload wiring', () => {
  it('buildChatTurn threads the pick into the chat_turn input, for BOTH executions', () => {
    // Task A3 moved the mapping out of App.tsx's two forked branches into the
    // single builder — which is exactly why the pick now reaches a quick chat
    // too, not just a background task.
    const builderSrc = readFileSync(path.resolve(__dirname, '../../../lib/buildChatTurn.ts'), 'utf8');
    expect(builderSrc).toMatch(/model_override:\s*payload\.model_override\s*\?\?\s*ctx\.modelOverride\s*\?\?\s*null/);
    // …and App.tsx injects the picker state at the composer call site and as
    // the builder's fallback context.
    const appSrc = readFileSync(path.resolve(__dirname, '../../../App.tsx'), 'utf8');
    expect(appSrc).toMatch(/model_override:\s*p\.model_override\s*\?\?\s*chatModelOverride/);
    expect(appSrc).toMatch(/modelOverride:\s*chatModelOverride/);
  });
});

// Wiring guard: which lifecycle a submission takes must be stated EXPLICITLY
// by every call site as `execution_mode`, never inferred from the absence of a
// sentinel. The retired encoding (`task_category: 'chat' | undefined`) made
// "background" the value you got by forgetting to say anything, so a new call
// site silently dispatched a task.
//
// Historical note: before that it was derived from `payload.mode === 'act'`,
// which was silently always true — Loquela defaults its internal `mode` to
// "act" for EVERY submission — so real chat messages always fell through to
// the full agentic pipeline.
describe('execution_mode submit-payload wiring', () => {
  it('buildChatTurn maps execution_mode to `execution`, defaulting to sync', () => {
    const builderSrc = readFileSync(path.resolve(__dirname, '../../../lib/buildChatTurn.ts'), 'utf8');
    expect(builderSrc).toMatch(/payload\.execution_mode === 'task'/);
    expect(builderSrc).toMatch(/'background'/);
    expect(builderSrc).toMatch(/'sync'/);
    expect(builderSrc).not.toMatch(/^\s*task_category:/m);
  });

  it("Loquela's composer send() emits execution_mode for BOTH toggle positions", () => {
    const loquelaSrc = readFileSync(
      path.resolve(__dirname, '../Loquela/Loquela.tsx'),
      'utf8',
    );
    expect(loquelaSrc).toMatch(/execution_mode:\s*executionMode,/);
    expect(loquelaSrc).not.toMatch(/^\s*task_category:/m);
  });

  it("/spawn's direct dispatch says execution_mode: 'task' rather than omitting it", () => {
    const appSrc = readFileSync(path.resolve(__dirname, '../../../App.tsx'), 'utf8');
    const spawnBlockMatch = appSrc.match(
      /base === '\/spawn'\) \{[\s\S]*?void handleLoquelaSubmit\(\{[\s\S]*?execution_mode: 'task'[\s\S]*?\}\);/,
    );
    expect(spawnBlockMatch).not.toBeNull();
    expect(spawnBlockMatch?.[0]).toMatch(/execution_mode: 'task'/);
  });
});
