import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import zedIcon from '../../assets/icons/zed.png';

export function ZedPage() {
    return (
        <CockpitAccountManagerView
            platformId="zed"
            platformLabel="Zed Cloud"
            platformIcon={zedIcon}
            noticeTitle="Zed Cloud Account & Token Management (click to expand/collapse)"
            permissionScope="read Zed authentication credentials, manage Zed language models (Claude 3.5 Sonnet, GPT-4o, Zed AI), and configure local editor preferences."
            networkScope="Token refresh and quota inspection connect to cloud.zed.dev and api.zed.dev. Session credentials are encrypted and stored locally."
        />
    );
}
