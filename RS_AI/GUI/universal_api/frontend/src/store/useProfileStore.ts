import { create } from 'zustand';
import {
    listProfiles,
    createNewProfile,
    removeProfile,
    duplicateProfile,
    bindAccountToProfile,
    launchProfileInstance,
    killRunningInstance,
    InstanceProfile,
} from '../../../bridge/profiles_bridge';
import { ipcErrorMessage } from '../../../bridge/ipc';

interface ProfileState {
    profiles: InstanceProfile[];
    loading: boolean;
    error: string | null;
    runningInstances: Record<string, number>; // profile_id -> pid
    fetchProfiles: () => Promise<void>;
    createProfile: (name: string, platformId: string, boundAccount?: string, args?: string[]) => Promise<InstanceProfile>;
    deleteProfile: (id: string) => Promise<void>;
    cloneProfile: (id: string, newName: string) => Promise<InstanceProfile>;
    bindAccount: (profileId: string, accountUid: string) => Promise<void>;
    launchInstance: (profileId: string, customBinPath?: string) => Promise<number>;
    stopInstance: (profileId: string) => Promise<void>;
}

export const useProfileStore = create<ProfileState>((set, get) => ({
    profiles: [],
    loading: false,
    error: null,
    runningInstances: {},

    fetchProfiles: async () => {
        set({ loading: true, error: null });
        try {
            const profiles = await listProfiles();
            set({ profiles, loading: false });
        } catch (e) {
            set({ error: ipcErrorMessage(e), loading: false });
        }
    },

    createProfile: async (name, platformId, boundAccount, args = []) => {
        set({ loading: true, error: null });
        try {
            const p = await createNewProfile({
                name,
                platform_id: platformId,
                bound_account_id: boundAccount || null,
                extra_args: args,
            });
            set((state) => ({ profiles: [...state.profiles, p], loading: false }));
            return p;
        } catch (e) {
            const err = ipcErrorMessage(e);
            set({ error: err, loading: false });
            throw new Error(err);
        }
    },

    deleteProfile: async (id: string) => {
        set({ loading: true, error: null });
        try {
            await removeProfile(id);
            set((state) => ({
                profiles: state.profiles.filter((p) => p.id !== id),
                loading: false,
            }));
        } catch (e) {
            set({ error: ipcErrorMessage(e), loading: false });
        }
    },

    cloneProfile: async (id: string, newName: string) => {
        set({ loading: true, error: null });
        try {
            const cloned = await duplicateProfile(id, newName);
            set((state) => ({ profiles: [...state.profiles, cloned], loading: false }));
            return cloned;
        } catch (e) {
            const err = ipcErrorMessage(e);
            set({ error: err, loading: false });
            throw new Error(err);
        }
    },

    bindAccount: async (profileId: string, accountUid: string) => {
        set({ loading: true, error: null });
        try {
            await bindAccountToProfile(profileId, accountUid);
            set((state) => ({
                profiles: state.profiles.map((p) =>
                    p.id === profileId ? { ...p, bound_account_id: accountUid } : p,
                ),
                loading: false,
            }));
        } catch (e) {
            const err = ipcErrorMessage(e);
            set({ error: err, loading: false });
            throw new Error(err);
        }
    },

    launchInstance: async (profileId: string, customBinPath?: string) => {
        try {
            const { pid } = await launchProfileInstance(profileId, customBinPath);
            set((state) => ({
                runningInstances: { ...state.runningInstances, [profileId]: pid },
            }));
            return pid;
        } catch (e) {
            const err = ipcErrorMessage(e);
            set({ error: err });
            throw new Error(err);
        }
    },

    stopInstance: async (profileId: string) => {
        const pid = get().runningInstances[profileId];
        if (pid) {
            try {
                await killRunningInstance(pid);
                set((state) => {
                    const next = { ...state.runningInstances };
                    delete next[profileId];
                    return { runningInstances: next };
                });
            } catch (e) {
                set({ error: ipcErrorMessage(e) });
            }
        }
    },
}));
