import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import qoderIcon from '../../assets/icons/qoder.png';

export function QoderPage() {
    return (
        <CockpitAccountManagerView
            platformId="qoder"
            platformLabel="Qoder"
            platformIcon={qoderIcon}
            noticeTitle="Qoder Multi-Account Management"
            permissionScope="Manage Qoder authentication tokens and profile bindings."
            networkScope="Direct API connections for session check. Tokens never leave local device."
        />
    );
}
