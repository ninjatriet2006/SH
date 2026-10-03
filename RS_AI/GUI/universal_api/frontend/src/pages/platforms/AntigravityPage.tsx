import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import antigravityIcon from '../../assets/icons/antigravity-menu.png';

export function AntigravityPage() {
    return (
        <CockpitAccountManagerView
            platformId="antigravity"
            platformLabel="Antigravity"
            platformIcon={antigravityIcon}
            noticeTitle="Google Antigravity Multi-Account Management (click to expand/collapse)"
            permissionScope="read Antigravity credentials from local storage and OAuth tokens (oauth_creds.json, state.vscdb), support switching between Pro and Free accounts."
            networkScope="Quota checking connects directly to Google Cloud / Vertex AI and Antigravity internal services. Session tokens are securely managed locally."
        />
    );
}
