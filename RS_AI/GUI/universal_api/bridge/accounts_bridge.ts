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

export async function refreshAntigravityQuota(uid: string): Promise<any> {
  return invokeIpc<any>('refresh_antigravity_quota', { uid });
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

export interface AntigravityQuotaBucket {
  bucket_id: string;
  label: string;
  remaining_percent: number;
  time_left: string;
}

export interface AntigravityOverviewCard {
  id: string;
  email: string;
  plan_tier: string;
  buckets: AntigravityQuotaBucket[];
}

export interface AntigravityOverview {
  current_account: AntigravityOverviewCard | null;
  recommended_account: AntigravityOverviewCard | null;
  total_accounts: number;
}

export interface ProviderStat {
  id: string;
  name: string;
  count: number;
  badge: string | null;
}

export async function getAntigravityOverview(): Promise<AntigravityOverview> {
  return invokeIpc<AntigravityOverview>('get_antigravity_overview');
}

export async function getProvidersOverview(): Promise<ProviderStat[]> {
  return invokeIpc<ProviderStat[]>('get_providers_overview');
}

export interface InstalledAppInfo {
  installed: boolean;
  name: string;
  version: string;
  exec_path: string;
  target_kind: string;
}

export async function getAntigravityInstalledVersionInfo(): Promise<InstalledAppInfo> {
  return invokeIpc<InstalledAppInfo>('get_antigravity_installed_version_info');
}

export async function getInstalledAppVersionInfo(
  platformId: string,
  variant?: string,
): Promise<InstalledAppInfo> {
  return invokeIpc<InstalledAppInfo>('get_installed_app_version_info', {
    platform_id: platformId,
    variant: variant || null,
  });
}

