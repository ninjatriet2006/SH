export interface SafetyFreedomView {
    value: string;
    title: string;
    known: boolean;
}

export function safetyFreedomView(
    score: number | null,
    confidence: number | null,
    evidence: string[],
    unknownLabel: string,
    confidenceLabel: string,
): SafetyFreedomView;
