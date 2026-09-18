import { invokeIpc } from './ipc';

export interface LoginStartResult {
    auth_url: string;
    realm: string;
}

export interface LoginAccount {
    uid: string;
    nickname: string;
    domain: string;
    checkin: string;
}

export type LoginPollResult = { status: 'pending'; account: null } | { status: 'done'; account: LoginAccount };

export async function loginStart(realm: string): Promise<LoginStartResult> {
    return await invokeIpc<LoginStartResult>('login_start', { realm });
}

export async function loginPoll(): Promise<LoginPollResult> {
    return await invokeIpc<LoginPollResult>('login_poll');
}

export async function loginCancel(): Promise<void> {
    await invokeIpc<Record<string, never>>('login_cancel');
}

export async function openLoginUrl(url: string): Promise<void> {
    await invokeIpc<Record<string, never>>('open_login_url', { url });
}
