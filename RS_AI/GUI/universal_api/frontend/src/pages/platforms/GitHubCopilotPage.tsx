import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import copilotIcon from '../../assets/icons/github-copilot.svg';

export function GitHubCopilotPage() {
    return (
        <CockpitAccountManagerView
            platformId="github_copilot"
            platformLabel="GitHub Copilot"
            platformIcon={copilotIcon}
            noticeTitle="GitHub Copilot Multi-Account Management (click to expand/collapse)"
            permissionScope="read GitHub Copilot token credentials from VS Code / CLI state (state.vscdb), support multi-account switching between Individual, Business, and Enterprise plans."
            networkScope="OAuth device authorization and token refresh connect directly to github.com and api.github.com. All tokens remain local."
        />
    );
}
