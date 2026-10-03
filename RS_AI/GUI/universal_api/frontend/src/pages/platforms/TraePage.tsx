import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import traeIcon from '../../assets/icons/trae.png';

export function TraePage() {
    return (
        <CockpitAccountManagerView
            platformId="trae"
            platformLabel="Trae"
            platformIcon={traeIcon}
            noticeTitle="Trae ByteDance AI IDE Multi-Account Management (click to expand/collapse)"
            permissionScope="read Trae user credentials and session cache (iCubeAuthInfo://usertag), support Trae International and Trae CN."
            networkScope="Authenticates with ByteDance Trae auth services (trae.ai / trae.cn) and retrieves active entitlement tokens."
        />
    );
}
