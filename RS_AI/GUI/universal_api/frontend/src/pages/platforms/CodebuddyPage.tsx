import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import codebuddyIcon from '../../assets/icons/codebuddy.png';

export function CodebuddyPage() {
    return (
        <CockpitAccountManagerView
            platformId="codebuddy"
            platformLabel="CodeBuddy"
            platformIcon={codebuddyIcon}
            noticeTitle="CodeBuddy Group Provider (Global & CN Multi-Region Management)"
            permissionScope="Manage both CodeBuddy Global (codebuddy.ai) and CodeBuddy CN (copilot.tencent.com) local access tokens."
            networkScope="Switch freely between Global OAuth / B3 endpoints and Tencent Copilot endpoints without cross-contamination."
        />
    );
}
