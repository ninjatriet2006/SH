import { beforeEach, describe, expect, it, vi } from 'vitest';

const { rawInvoke } = vi.hoisted(() => ({ rawInvoke: vi.fn() }));
vi.mock('@tauri-apps/api/core', () => ({ invoke: rawInvoke }));

import { invoke, unwrap } from '../../bridge/ipc';

describe('IPC facade', () => {
  beforeEach(() => rawInvoke.mockReset());

  it('sends Req and unwraps Res without changing command names', async () => {
    rawInvoke.mockResolvedValue({ schema_version: 1, request_id: null, data: ['ok'] });
    await expect(invoke<string[]>('list_files', { path: '/', pane: undefined })).resolves.toEqual(['ok']);
    expect(rawInvoke).toHaveBeenCalledWith('list_files', {
      request: { schema_version: 1, request_id: null, payload: { path: '/', pane: null } },
    });
  });

  it('normalizes legacy camelCase arguments at the facade', async () => {
    rawInvoke.mockResolvedValue({ schema_version: 1, request_id: null, data: null });
    await invoke('fs_rename', { oldPath: '/a', newPath: '/b' });
    expect(rawInvoke.mock.calls[0][1].request.payload).toEqual({ old_path: '/a', new_path: '/b' });
  });

  it('rejects malformed response envelopes', () => {
    expect(() => unwrap({ schema_version: 2, request_id: null, data: true } as never)).toThrow(/Malformed/);
  });
});
