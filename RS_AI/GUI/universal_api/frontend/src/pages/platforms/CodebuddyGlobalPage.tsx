import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import codebuddyIcon from '../../assets/icons/codebuddy.png';

export function CodebuddyGlobalPage() {
    return (
        <CockpitAccountManagerView
            platformId="codebuddy_global"
            platformLabel="CodeBuddy Global"
            platformIcon={codebuddyIcon}
            noticeTitle="CodeBuddy Global Account Info (click to expand/collapse)"
            permissionScope="read CodeBuddy auth database (state.vscdb), call system credential capabilities (macOS Keychain / Windows DPAPI / Linux Secret Service) for decrypt/write-back."
            networkScope="OAuth login and token refresh require network requests to codebuddy.ai; quota queries call billing APIs. No local keys or credentials are uploaded."
        />
    );
}
