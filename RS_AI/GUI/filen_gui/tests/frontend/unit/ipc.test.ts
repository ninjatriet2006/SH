import { describe, expect, it, vi } from 'vitest';
import { invoke, request } from '../../../frontend/src/ipc';

describe('A.1 IPC facade', () => {
  it('sends Req and unwraps Res.data', async () => {
    const adapter = { invoke: vi.fn().mockResolvedValue({ schema_version: 1, request_id: null, data: 7 }) };
    await expect(invoke<number>('transfer_enqueue', { kind: 'copy' }, adapter)).resolves.toBe(7);
    expect(adapter.invoke).toHaveBeenCalledWith('transfer_enqueue', { request: request({ kind: 'copy' }) });
  });

  it('normalizes malformed rejection and rejects malformed response', async () => {
    const badError = { invoke: vi.fn().mockRejectedValue('boom') };
    await expect(invoke('x', {}, badError)).rejects.toMatchObject({ code: 'internal', message: 'boom', details: null });
    const badResponse = { invoke: vi.fn().mockResolvedValue({ data: 1 }) };
    await expect(invoke('x', {}, badResponse)).rejects.toMatchObject({ code: 'internal' });
  });

  it('rejects omitted nullable payload fields before invoking', async () => {
    const adapter = { invoke: vi.fn() };
    await expect(invoke('x', { account: undefined }, adapter)).rejects.toMatchObject({ code: 'invalid_argument' });
    expect(adapter.invoke).not.toHaveBeenCalled();
  });
});
