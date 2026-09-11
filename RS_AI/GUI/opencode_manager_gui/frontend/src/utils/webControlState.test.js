import assert from 'node:assert/strict';
import test from 'node:test';
import {
    copyLocalWebUrl, isLocalWebUrl, isNewerWebStatus, openLocalWebUrl, webControlMatrix,
} from './webControlState.js';

const status = (state, url = null, generation = 1, revision = 1, hasOwnedChild = state === 'running') => ({
    state, url, error: null, has_owned_child: hasOwnedChild, generation, revision,
});

test('only exact HTTP loopback URLs are accepted', () => {
    assert.equal(isLocalWebUrl('http://127.0.0.1:4096/'), true);
    assert.equal(isLocalWebUrl('http://localhost:4096/'), true);
    for (const invalid of [
        'https://127.0.0.1:4096/', 'http://0.0.0.0:4096/',
        'http://example.com:4096/', 'http://localhost:4096/path',
        'http://user@localhost:4096/', 'not a url', null,
    ]) assert.equal(isLocalWebUrl(invalid), false, String(invalid));
});

test('disabled matrix covers every lifecycle state and invalid URL', () => {
    assert.deepEqual(webControlMatrix(status('stopped')), {
        startDisabled: false, stopDisabled: true, copyDisabled: true, openDisabled: true,
    });
    assert.deepEqual(webControlMatrix(status('error')), {
        startDisabled: false, stopDisabled: true, copyDisabled: true, openDisabled: true,
    });
    assert.deepEqual(webControlMatrix(status('error', null, 1, 1, true)), {
        startDisabled: false, stopDisabled: false, copyDisabled: true, openDisabled: true,
    });
    for (const state of ['starting', 'stopping']) {
        assert.deepEqual(webControlMatrix(status(state)), {
            startDisabled: true, stopDisabled: true, copyDisabled: true, openDisabled: true,
        });
    }
    assert.deepEqual(webControlMatrix(status('running', 'http://127.0.0.1:4000/')), {
        startDisabled: true, stopDisabled: false, copyDisabled: false, openDisabled: false,
    });
    assert.equal(webControlMatrix(status('running', 'http://evil.test:4000/')).openDisabled, true);
    assert.equal(webControlMatrix(status('running', 'http://127.0.0.1:4000/'), true).stopDisabled, true);
});

test('generation and revision reject stale poll and mutation responses', () => {
    const current = status('running', 'http://localhost:2/', 4, 9);
    assert.equal(isNewerWebStatus(status('starting', null, 3, 99), current), false);
    assert.equal(isNewerWebStatus(status('starting', null, 4, 8), current), false);
    assert.equal(isNewerWebStatus(status('running', 'http://localhost:2/', 4, 9), current), true);
    assert.equal(isNewerWebStatus(status('starting', null, 5, 1), current), true);
});

test('copy/open actions validate first and propagate platform failures', async () => {
    const valid = 'http://localhost:7777/';
    let received = '';
    await copyLocalWebUrl(valid, async url => { received = url; });
    assert.equal(received, valid);
    await openLocalWebUrl(valid, async url => { received = url; });
    assert.equal(received, valid);
    await assert.rejects(copyLocalWebUrl('http://evil.test:7/', async () => {}), /invalid_localhost_url/);
    await assert.rejects(copyLocalWebUrl(valid, async () => { throw new Error('clipboard denied'); }), /clipboard denied/);
    await assert.rejects(openLocalWebUrl(valid, async () => { throw new Error('browser missing'); }), /browser missing/);
});
