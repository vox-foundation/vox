import { describe, expect, it, vi } from 'vitest';
import { emptyLiveState } from './axisDrive';
import {
  assertPinSelectable,
  driveVerbNeedsCatalog,
  handleDriveRequest,
  interpretDriveSubmit,
} from './useDriveBus';
import type { PickerModel, ProviderStatus } from './modelPicker';

const localDown: ProviderStatus[] = [
  {
    provider: 'VoxLocal',
    key_present: false,
    is_local: true,
    local_reachable: false,
    local_models: [],
  },
];

const localModel: PickerModel[] = [
  { id: 'mens/e2e-smoke-metal', label: 'metal', provider: 'VoxLocal', providerType: 'local' },
];

describe('interpretDriveSubmit', () => {
  it('matrix: null / ok false / error text / ok true / bracket error', () => {
    expect(interpretDriveSubmit(null)).toEqual({
      lastError: 'submit_unspecified',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ ok: false, error: 'nope' })).toEqual({
      lastError: 'nope',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ ok: false })).toEqual({
      lastError: 'submit_failed',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ error: 'soft' })).toEqual({
      lastError: 'soft',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ ok: true, text: 'hello' })).toEqual({
      lastError: null,
      assistantText: 'hello',
    });
    expect(interpretDriveSubmit({ ok: true, text: '[error: boom]' })).toEqual({
      lastError: '[error: boom]',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ ok: true, text: 'error: boom' })).toEqual({
      lastError: 'error: boom',
      assistantText: null,
    });
    expect(interpretDriveSubmit({ ok: true })).toEqual({
      lastError: null,
      assistantText: null,
    });
  });
});

describe('handleDriveRequest', () => {
  it('send calls submit with description and maps sync→chat', async () => {
    const submit = vi.fn(async () => undefined);
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: localDown,
      submit,
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(submit).toHaveBeenCalledTimes(1);
    expect(submit.mock.calls[0]?.[0]).toMatchObject({
      description: 'ping',
      execution_mode: 'chat',
    });
    expect(JSON.stringify(res)).not.toMatch(/tool_call|9745/);
    expect(res.plane).toBe('live');
    expect(res.state.last_error).toBe('submit_unspecified');
  });

  it('empty send is empty_text', async () => {
    const submit = vi.fn();
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: [],
      submit,
      req: { id: '1', verb: 'send', body: { text: '' } },
    });
    expect(res.status).toBe(400);
    expect(res.error).toBe('empty_text');
    expect(submit).not.toHaveBeenCalled();
  });

  it('set model_override calls setChatModelOverride', async () => {
    const setChatModelOverride = vi.fn();
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: localModel,
      statuses: [
        {
          provider: 'VoxLocal',
          key_present: false,
          is_local: true,
          local_reachable: true,
          local_models: ['e2e-smoke-metal'],
        },
      ],
      setters: { setChatModelOverride },
      submit: vi.fn(),
      req: { id: '1', verb: 'set', body: { model_override: 'mens/e2e-smoke-metal' } },
    });
    expect(setChatModelOverride).toHaveBeenCalledWith('mens/e2e-smoke-metal');
    expect(res.status).toBe(200);
    expect(res.state.pin).toBe('mens/e2e-smoke-metal');
  });

  it('unselectable set does not call setChatModelOverride', async () => {
    const setChatModelOverride = vi.fn();
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: localModel,
      statuses: localDown,
      setters: { setChatModelOverride },
      submit: vi.fn(),
      req: { id: '1', verb: 'set', body: { model_override: 'mens/e2e-smoke-metal' } },
    });
    expect(res.status).toBe(409);
    expect(res.error).toBe('model_not_selectable');
    expect(setChatModelOverride).not.toHaveBeenCalled();
  });

  it('unselectable pin with fail policy is Drive-only 409', async () => {
    const state = {
      ...emptyLiveState(),
      knobs: { ...emptyLiveState().knobs, model_override: 'mens/e2e-smoke-metal', pin_policy: 'fail' as const },
      pin: 'mens/e2e-smoke-metal',
    };
    const blocked = assertPinSelectable(state, localModel, localDown);
    expect(blocked?.status).toBe(409);
    expect(blocked?.error).toBe('model_not_selectable');
    const res = await handleDriveRequest({
      state,
      models: localModel,
      statuses: localDown,
      submit: vi.fn(),
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(res.status).toBe(409);
    expect(res.error).toBe('model_not_selectable');
  });

  it('send records last_error and an assistant error bubble when submit fails', async () => {
    const submit = vi.fn(async () => ({
      ok: false as const,
      error: 'load tokenizer: No such file or directory (os error 2)',
    }));
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: localDown,
      submit,
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(res.status).toBe(200);
    expect(res.state.last_error).toMatch(/tokenizer/);
    expect(res.state.bubbles).toEqual([
      { role: 'user', content: 'ping' },
      {
        role: 'assistant',
        content: 'load tokenizer: No such file or directory (os error 2)',
        error: true,
      },
    ]);
  });

  it('send records an assistant bubble when submit returns text', async () => {
    const submit = vi.fn(async () => ({ ok: true as const, text: 'hello from axis' }));
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: localDown,
      submit,
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(res.state.last_error).toBeNull();
    expect(res.state.bubbles.at(-1)).toEqual({ role: 'assistant', content: 'hello from axis' });
  });

  it('send maps a thrown submit to last_error', async () => {
    const submit = vi.fn(async () => {
      throw new Error('boom');
    });
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [],
      statuses: localDown,
      submit,
      req: { id: '1', verb: 'send', body: { text: 'ping' } },
    });
    expect(res.state.last_error).toBe('boom');
    expect(res.state.bubbles.at(-1)).toMatchObject({ role: 'assistant', error: true, content: 'boom' });
  });

  it('send clears prior events and records submit_err', async () => {
    const state = emptyLiveState();
    state.events = [{ seq: 1, ts_ms: 1, kind: 'old', turn_id: 'old', text: 'x' }];
    state.next_seq = 2;
    const res = await handleDriveRequest({
      req: { id: '1', verb: 'send', body: { text: 'hi' } },
      state,
      models: [{ id: 'openrouter/auto', provider: 'OpenRouter', providerType: 'OpenRouter' } as any],
      statuses: [{ provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null } as any],
      submit: async () => ({ ok: false, error: 'boom' }),
    });
    expect(res.state.events.some((e: any) => e.kind === 'old')).toBe(false);
    expect(res.state.events.some((e: any) => e.kind === 'submit_err')).toBe(true);
    expect(res.state.last_error).toBe('boom');
    expect(res.state.last_turn_id).toBeTruthy();
  });

  it('send records submit_ok with assistant text', async () => {
    const res = await handleDriveRequest({
      req: { id: '1', verb: 'send', body: { text: 'hi' } },
      state: emptyLiveState(),
      models: [],
      statuses: [],
      submit: async () => ({ ok: true, text: 'hello-assistant' }),
    });
    expect(res.state.last_error).toBeNull();
    expect(res.state.events.some((e: any) => e.kind === 'submit_ok' && e.text === 'hello-assistant')).toBe(true);
  });

  it('mutation: send path references clearDriveEvents and appendDriveEvent', () => {
    const src = handleDriveRequest.toString();
    expect(src).toMatch(/clearDriveEvents/);
    expect(src).toMatch(/appendDriveEvent/);
  });

  it('state, set, and send require catalog IPC; show does not', () => {
    expect(driveVerbNeedsCatalog('state')).toBe(true);
    expect(driveVerbNeedsCatalog('show')).toBe(false);
    expect(driveVerbNeedsCatalog('set')).toBe(true);
    expect(driveVerbNeedsCatalog('send')).toBe(true);
  });

  it('state includes OpenRouter provider_type in its catalog snapshot', async () => {
    const res = await handleDriveRequest({
      state: emptyLiveState(),
      models: [
        { id: 'openrouter/auto', label: 'auto', provider: 'OpenRouter', providerType: 'OpenRouter' },
      ],
      statuses: [{ provider: 'OpenRouter', key_present: true, is_local: false, local_reachable: null }],
      submit: vi.fn(),
      req: { id: '1', verb: 'state', body: {} },
    });
    expect(res.state.catalog[0]).toMatchObject({
      id: 'openrouter/auto',
      provider_type: 'OpenRouter',
    });
  });

  it('mutation: deleting the selectable check would miss the 409', () => {
    // Guard the Drive-only 409: if assertPinSelectable is removed, this fails.
    expect(assertPinSelectable.toString()).toMatch(/model_not_selectable|selectable/);
    const src = handleDriveRequest.toString();
    expect(src).toContain('assertPinSelectable');
  });
});
