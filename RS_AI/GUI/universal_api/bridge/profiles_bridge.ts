import { invokeIpc } from './ipc';

export interface HardwareFingerprint {
    machine_id: string;
    mac_machine_id: string;
    dev_device_id: string;
    sqm_id: string;
}

export interface InstanceProfile {
    id: string;
    name: string;
    platform_id: string;
    user_data_dir: string;
    userDataDir?: string;
    working_dir?: string | null;
    workingDir?: string | null;
    bound_account_id?: string | null;
    bindAccountId?: string | null;
    extra_args?: any;
    extraArgs?: any;
    hardware_fingerprint?: HardwareFingerprint;
    created_at?: string;
    createdAt?: string;
    last_launched_at?: string | null;
    lastPid?: number | null;
    last_pid?: number | null;
    is_default?: boolean;
    isDefault?: boolean;
    is_running?: boolean;
    isRunning?: boolean;
    launch_mode?: string;
    launchMode?: string;
}

export async function listProfiles(): Promise<InstanceProfile[]> {
    return invokeIpc<InstanceProfile[]>('list_profiles');
}

export async function createNewProfile(payload: {
    name: string;
    platform_id: string;
    bound_account_id?: string | null;
    extra_args?: string[];
}): Promise<InstanceProfile> {
    return invokeIpc<InstanceProfile>('create_new_profile', payload);
}

export async function removeProfile(id: string): Promise<void> {
    return invokeIpc<void>('remove_profile', { id });
}

export async function duplicateProfile(id: string, new_name: string): Promise<InstanceProfile> {
    return invokeIpc<InstanceProfile>('duplicate_profile', { id, new_name });
}

export async function bindAccountToProfile(profile_id: string, account_uid: string): Promise<void> {
    return invokeIpc<void>('bind_account_to_profile', { profile_id, account_uid });
}

export async function launchProfileInstance(
    profile_id: string,
    custom_binary_path?: string | null,
): Promise<{ pid: number; profile_id: string }> {
    return invokeIpc<{ pid: number; profile_id: string }>('launch_profile_instance', {
        profile_id,
        custom_binary_path: custom_binary_path || null,
    });
}

export async function killRunningInstance(pid: number): Promise<void> {
    return invokeIpc<void>('kill_running_instance', { pid });
}

// ==========================================
// Platform-Scoped Instances (Cockpit 1:1 Parity)
// ==========================================

export async function listPlatformInstances(platform: string): Promise<InstanceProfile[]> {
    return invokeIpc<InstanceProfile[]>('list_platform_instances', { platform });
}

export async function createPlatformInstance(payload: {
    platform: string;
    name: string;
    init_mode?: string;
    source_instance_id?: string | null;
    existing_dir?: string | null;
    bind_account_id?: string | null;
    extra_args?: string | null;
}): Promise<InstanceProfile> {
    return invokeIpc<InstanceProfile>('create_platform_instance', payload);
}

export async function updatePlatformInstance(payload: {
    platform: string;
    instance_id: string;
    name?: string;
    bind_account_id?: string | null;
    extra_args?: string;
}): Promise<void> {
    return invokeIpc<void>('update_platform_instance', payload);
}

export async function deletePlatformInstance(platform: string, instance_id: string): Promise<void> {
    return invokeIpc<void>('delete_platform_instance', { platform, instance_id });
}

export async function launchPlatformInstance(
    platform: string,
    instance_id: string,
    custom_binary_path?: string | null,
): Promise<{ pid: number; instance_id: string }> {
    return invokeIpc<{ pid: number; instance_id: string }>('launch_platform_instance', {
        platform,
        instance_id,
        custom_binary_path: custom_binary_path || null,
    });
}

export async function stopPlatformInstance(
    platform: string,
    instance_id: string,
    pid?: number | null,
): Promise<void> {
    return invokeIpc<void>('stop_platform_instance', {
        platform,
        instance_id,
        pid: pid ?? null,
    });
}

export async function openInstanceFolder(path: string): Promise<void> {
    return invokeIpc<void>('open_instance_folder', { path });
}
