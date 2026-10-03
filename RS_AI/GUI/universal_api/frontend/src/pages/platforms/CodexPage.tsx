import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import codexIcon from '../../assets/icons/codex.svg';

export function CodexPage() {
    return (
        <CockpitAccountManagerView
            platformId="codex"
            platformLabel="Codex"
            platformIcon={codexIcon}
            noticeTitle="OpenAI Codex / ChatGPT Multi-Account Management"
            permissionScope="Manage OpenAI access tokens, refresh tokens, and session keys."
            networkScope="Direct quota queries against OpenAI API endpoints. All tokens remain offline/local."
        />
    );
}
