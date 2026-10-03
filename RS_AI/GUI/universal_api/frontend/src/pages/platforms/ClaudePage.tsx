import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import claudeIcon from '../../assets/icons/claude.png';

export function ClaudePage() {
    return (
        <CockpitAccountManagerView
            platformId="claude"
            platformLabel="Claude"
            platformIcon={claudeIcon}
            noticeTitle="Anthropic Claude Multi-Account & Session Management"
            permissionScope="Manage Claude CLI / Web session credentials and token vaults."
            networkScope="Direct API / Web session checking with Anthropic. Credentials stay local."
        />
    );
}
