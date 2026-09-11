import assert from 'node:assert/strict';
import test from 'node:test';
import { safetyFreedomView } from './safetyFreedom.js';

test('ArbiterVerdict DTO JSON preserves known and unknown safety fields', () => {
    const known = { safety_freedom: 100, safety_confidence: 90, safety_evidence: ['documented'], overall_version: 2 };
    const unknown = { safety_freedom: null, safety_confidence: null, safety_evidence: [], overall_version: 2 };
    assert.deepEqual(JSON.parse(JSON.stringify(known)), known);
    assert.deepEqual(JSON.parse(JSON.stringify(unknown)), unknown);
});

test('UI projection renders known score with evidence and unknown as dash', () => {
    assert.deepEqual(safetyFreedomView(null, null, [], 'Unknown', 'Confidence'), {
        value: '—', title: 'Unknown', known: false,
    });
    assert.deepEqual(safetyFreedomView(87.5, 80, ['Observed behavior'], 'Unknown', 'Confidence'), {
        value: '87.5', title: 'Confidence: 80 · Observed behavior', known: true,
    });
    for (const malformed of [
        [100, null, ['evidence']],
        [100, 90, []],
        [100, 90, ['']],
    ]) {
        assert.deepEqual(safetyFreedomView(...malformed, 'Unknown', 'Confidence'), {
            value: '—', title: 'Unknown', known: false,
        });
    }
});
