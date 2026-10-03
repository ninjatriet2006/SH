import { CockpitAccountManagerView } from '../components/CockpitAccountManagerView';
import codebuddyIcon from '../assets/icons/codebuddy.png';

export function AccountsPage() {
    return (
        <CockpitAccountManagerView
            platformId="all"
            platformLabel="All Vault Accounts"
            platformIcon={codebuddyIcon}
            noticeTitle="Cockpit Universal Account Vault (click to expand/collapse)"
            permissionScope="Quản lý toàn bộ danh tính AI IDE (CodeBuddy CN, CodeBuddy Global, Zed, Antigravity). Token được lưu mã hóa an toàn trong SQLite cục bộ."
            networkScope="OAuth xác thực qua luồng chuẩn IDE desktop; Quota truy vấn trực tiếp từ API billing của nhà cung cấp."
        />
    );
}
