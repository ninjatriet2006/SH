import type { WebStatus } from '../../../bridge/types';

export function isLocalWebUrl(url: unknown): url is string;
export function isNewerWebStatus(candidate: WebStatus, current: WebStatus): boolean;
export function webControlMatrix(status: WebStatus, mutationBusy?: boolean): {
    startDisabled: boolean;
    stopDisabled: boolean;
    copyDisabled: boolean;
    openDisabled: boolean;
};
export function copyLocalWebUrl(url: unknown, writeText: (url: string) => Promise<void>): Promise<void>;
export function openLocalWebUrl(url: unknown, openUrl: (url: string) => Promise<void>): Promise<void>;
