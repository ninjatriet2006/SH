import { describe, it, expect, vi, beforeEach } from 'vitest';
import { setClipboard, getClipboard, clearClipboard, hasClipboard, pasteTo } from '../../../../frontend/src/features/clipboard';
import * as fileOps from '../../../../frontend/src/services/fileOps';

vi.mock('../../../../frontend/src/services/fileOps', () => ({
  cpBatch: vi.fn(),
  moveLocal: vi.fn(),
  upload: vi.fn(),
}));

const mocked = vi.mocked(fileOps);

beforeEach(() => {
  clearClipboard();
  vi.clearAllMocks();
});

describe('clipboard state', () => {
  it('set/get/clear/has', () => {
    expect(hasClipboard()).toBe(false);
    setClipboard('copy', [{ pane: 'left', path: '/a/b.txt' }]);
    expect(hasClipboard()).toBe(true);
    expect(getClipboard()).toEqual({ mode: 'copy', items: [{ pane: 'left', path: '/a/b.txt' }] });
    clearClipboard();
    expect(hasClipboard()).toBe(false);
    expect(getClipboard().items).toEqual([]);
  });
});

describe('pasteTo', () => {
  const refresh = vi.fn(async () => {});

  it('does nothing when clipboard empty', async () => {
    await pasteTo('left', '/dst', refresh);
    expect(mocked.cpBatch).not.toHaveBeenCalled();
    expect(refresh).not.toHaveBeenCalled();
  });

  it('copy to local pane calls cpBatch', async () => {
    setClipboard('copy', [{ pane: 'left', path: '/src/a.txt' }]);
    await pasteTo('left', '/dst', refresh);
    expect(mocked.cpBatch).toHaveBeenCalledWith(['/src/a.txt'], '/dst', true);
    expect(refresh).toHaveBeenCalledWith('left', '/dst');
  });

  it('cut to local pane calls moveLocal per item and clears clipboard', async () => {
    setClipboard('cut', [{ pane: 'left', path: '/src/a.txt' }]);
    await pasteTo('left', '/dst', refresh);
    expect(mocked.moveLocal).toHaveBeenCalledWith('/src/a.txt', '/dst/a.txt');
    expect(hasClipboard()).toBe(false);
  });

  it('copy to cloud pane calls upload per item and keeps clipboard', async () => {
    setClipboard('copy', [{ pane: 'left', path: '/src/a.txt' }]);
    await pasteTo('right', '/remote', refresh);
    expect(mocked.upload).toHaveBeenCalledWith('/src/a.txt', '/remote/a.txt');
    expect(hasClipboard()).toBe(true);
  });

  it('swallows errors (no throw) and does not refresh on failure', async () => {
    mocked.cpBatch.mockRejectedValueOnce(new Error('boom'));
    setClipboard('copy', [{ pane: 'left', path: '/src/a.txt' }]);
    await expect(pasteTo('left', '/dst', refresh)).resolves.toBeUndefined();
    expect(refresh).not.toHaveBeenCalled();
  });
});