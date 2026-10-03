import { CockpitAccountManagerView } from '../../components/CockpitAccountManagerView';
import cursorIcon from '../../assets/icons/cursor-menu.png';

export function CursorPage() {
    return (
        <CockpitAccountManagerView
            platformId="cursor"
            platformLabel="Cursor"
            platformIcon={cursorIcon}
            noticeTitle="Cursor Pro / Business Account Management (click to expand/collapse)"
            permissionScope="read Cursor authentication state from local Cursor storage (cursorAuth/*, state.vscdb), inject switch tokens into local editor."
            networkScope="Token verification queries api2.cursor.sh and Stripe subscription status. Session cookies remain on device."
        />
    );
}
