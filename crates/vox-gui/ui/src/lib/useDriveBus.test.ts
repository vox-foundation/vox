import { describe, expect, it, vi } from 'vitest';
import { emptyLiveState } from './axisDrive';
import { assertPinSelectable, handleDriveRequest } from './useDriveBus';
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

  it('mutation: deleting the selectable check would miss the 409', () => {
    // Guard the Drive-only 409: if assertPinSelectable is removed, this fails.
    expect(assertPinSelectable.toString()).toMatch(/model_not_selectable|selectable/);
    const src = handleDriveRequest.toString();
    expect(src).toContain('assertPinSelectable');
  });
});
