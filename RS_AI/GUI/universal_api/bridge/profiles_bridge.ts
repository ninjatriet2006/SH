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
    bound_account_id: string | null;
    extra_args: string[];
    hardware_fingerprint: HardwareFingerprint;
    created_at: string;
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
