import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import kiroIcon from '../../assets/icons/kiro-menu.png';

export function KiroPage() {
    return (
        <CockpitAccountManagerView
            platformId="kiro"
            platformLabel="Kiro"
            platformIcon={kiroIcon}
            noticeTitle="Kiro Multi-Account Management"
            permissionScope="Manage Kiro session tokens and hardware fingerprint isolation."
            networkScope="Direct API connections for session check. Tokens never leave local device."
        />
    );
}
