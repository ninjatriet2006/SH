import { invokeIpc } from './ipc';
import type { WebStatus } from './types';

export interface WebCommandContract {
    web_status: { payload: Record<string, never>; response: WebStatus };
    web_start: { payload: Record<string, never>; response: WebStatus };
    web_stop: { payload: Record<string, never>; response: WebStatus };
    launch_terminal: { payload: Record<string, never>; response: void };
    open_web_url: { payload: { url: string }; response: void };
}

function invokeWeb<K extends keyof WebCommandContract>(
    command: K,
    payload: WebCommandContract[K]['payload'],
): Promise<WebCommandContract[K]['response']> {
    return invokeIpc<WebCommandContract[K]['response'], WebCommandContract[K]['payload']>(command, payload);
}

export const getWebStatus = () => invokeWeb('web_status', {});
export const startWeb = () => invokeWeb('web_start', {});
export const stopWeb = () => invokeWeb('web_stop', {});
export const launchOpenCodeTerminal = () => invokeWeb('launch_terminal', {});
export const openWebUrl = (url: string) => invokeWeb('open_web_url', { url });
