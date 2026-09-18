import { invokeIpc } from './ipc';
import type { AccountInfo } from './types';

export async function listAccounts(): Promise<AccountInfo[]> {
  return invokeIpc<AccountInfo[]>('list_accounts');
}
export async function addAccount(filePath: string): Promise<void> {
  await invokeIpc<Record<string, never>>('add_account', { auth_file_path: filePath });
}
export async function removeAccount(uid: string): Promise<void> {
  await invokeIpc<Record<string, never>>('remove_account', { uid });
}
export async function disableAccount(uid: string): Promise<void> {
  await invokeIpc<Record<string, never>>('disable_account', { uid, reason: 'Disabled from GUI' });
}
export async function enableAccount(uid: string): Promise<void> {
  await invokeIpc<Record<string, never>>('enable_account', { uid });
}
export async function updateAccountRouting(
  uid: string,
  proxyUrl?: string | null,
  userAgent?: string | null,
  customHeaders?: Record<string, string> | null
): Promise<void> {
  await invokeIpc<Record<string, never>>('update_account_routing', {
    uid,
    proxy_url: proxyUrl,
    user_agent: userAgent,
    custom_headers: customHeaders,
  });
}

export interface TestAccountResult {
  uid: string;
  ok: boolean;
  remain: number;
  message: string;
}

export async function testAccount(uid: string): Promise<TestAccountResult> {
  return invokeIpc<TestAccountResult>('test_account', { uid });
}

export interface ProbeAccountResult {
  uid: string;
  quota_ok: boolean;
  remain: number;
  quota_message: string;
  models: { ok: boolean; count: number; has_glm52: boolean; message: string };
  chat: { ok: boolean; http_status: number; preview: string; message: string };
}

export async function probeAccount(uid: string): Promise<ProbeAccountResult> {
  return invokeIpc<ProbeAccountResult>('probe_account', { uid });
}
