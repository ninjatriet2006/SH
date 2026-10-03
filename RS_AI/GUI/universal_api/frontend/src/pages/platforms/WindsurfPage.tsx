import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import windsurfIcon from '../../assets/icons/windsurf.svg';

export function WindsurfPage() {
    return (
        <CockpitAccountManagerView
            platformId="windsurf"
            platformLabel="Windsurf"
            platformIcon={windsurfIcon}
            noticeTitle="Windsurf / Codeium Cascade Account Management (click to expand/collapse)"
            permissionScope="read Codeium / Windsurf credentials (windsurf_auth-%), manage Flow Action credits, fast requests, and Cascade quotas."
            networkScope="Queries api.codeium.com for credit verification and plan status. No code or context uploaded."
        />
    );
}
