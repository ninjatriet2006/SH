import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import test from 'node:test';

const read = path => readFile(new URL(path, import.meta.url), 'utf8');

test('ModelsPage renders roomy link-left controls-right web panel', async () => {
    const [page, component, css] = await Promise.all([
        read('../pages/ModelsPage.tsx'), read('../components/OpenCodeControls.tsx'), read('../index.css'),
    ]);
    assert.match(page, /<OpenCodeControls onError=\{setNotice\}/);
    assert.match(component, /opencode-controls__link/);
    assert.match(component, /opencode-controls__actions/);
    assert.match(css, /\.opencode-controls\s*\{[^}]*justify-content:\s*space-between/s);
    assert.match(css, /\.opencode-controls\s*\{[^}]*gap:\s*3rem/s);
});

test('EN and VI dictionaries contain all lifecycle and action labels', async () => {
    const [en, vi] = await Promise.all([
        read('../../../langs/en.json').then(JSON.parse), read('../../../langs/vi.json').then(JSON.parse),
    ]);
    for (const key of [
        'web_title', 'web_start', 'web_stop', 'web_copy', 'web_open', 'terminal_open',
        'web_state_stopped', 'web_state_starting', 'web_state_running',
        'web_state_stopping', 'web_state_error',
    ]) {
        assert.equal(typeof en.models[key], 'string', `EN ${key}`);
        assert.equal(typeof vi.models[key], 'string', `VI ${key}`);
    }
});
