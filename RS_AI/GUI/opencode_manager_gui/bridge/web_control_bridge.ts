import { invoke } from '@tauri-apps/api/core';
import type { WebStatus } from './types';

export const getWebStatus = () => invoke<WebStatus>('web_status');
export const startWeb = () => invoke<WebStatus>('web_start');
export const stopWeb = () => invoke<WebStatus>('web_stop');
export const launchOpenCodeTerminal = () => invoke<void>('launch_terminal');
export const openWebUrl = (url: string) => invoke<void>('open_web_url', { url });
