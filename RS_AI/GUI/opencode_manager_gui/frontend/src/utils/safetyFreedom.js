/**
 * Pure UI projection so known/unknown behavior stays independently testable.
 * @param {number | null} score
 * @param {number | null} confidence
 * @param {string[]} evidence
 * @param {string} unknownLabel
 * @param {string} confidenceLabel
 */
export function safetyFreedomView(score, confidence, evidence, unknownLabel, confidenceLabel) {
    const validScore = Number.isFinite(score) && score >= 0 && score <= 100;
    const validConfidence = Number.isFinite(confidence) && confidence >= 0 && confidence <= 100;
    const validEvidence = Array.isArray(evidence)
        && evidence.length >= 1
        && evidence.length <= 3
        && evidence.every(item => typeof item === 'string' && item.trim().length > 0 && [...item.trim()].length <= 200);
    if (!validScore || !validConfidence || !validEvidence) {
        return { value: '—', title: unknownLabel, known: false };
    }
    const details = evidence.map(item => item.trim());
    details.unshift(`${confidenceLabel}: ${confidence}`);
    return { value: Number.isInteger(score) ? String(score) : score.toFixed(1), title: details.join(' · '), known: true };
}
