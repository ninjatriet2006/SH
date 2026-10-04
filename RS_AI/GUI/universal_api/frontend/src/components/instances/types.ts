import type { InstanceProfile } from '../../../../bridge/profiles_bridge';
import type { AccountInfo } from '../../../../bridge/types';

export interface PlatformInstancesContentProps {
    platformId: string;
    platformLabel: string;
    accounts: AccountInfo[];
    onSwitchAccount?: (account: AccountInfo) => void;
}

export type { InstanceProfile, AccountInfo };
