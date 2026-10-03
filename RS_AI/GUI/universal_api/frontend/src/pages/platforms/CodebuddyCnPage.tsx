import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import codebuddyIcon from '../../assets/icons/codebuddy.png';

export function CodebuddyCnPage() {
    return (
        <CockpitAccountManagerView
            platformId="codebuddy_cn"
            platformLabel="CodeBuddy CN"
            platformIcon={codebuddyIcon}
            noticeTitle="CodeBuddy CN Account Info (click to expand/collapse)"
            permissionScope="read CodeBuddy CN auth database (state.vscdb), call system credential capabilities (macOS Keychain / Windows DPAPI / Linux Secret Service) for decrypt/write-back."
            networkScope="OAuth login and token refresh require network requests to codebuddy.cn and copilot.tencent.com; quota queries call billing APIs. No local keys or credentials are uploaded."
        />
    );
}
