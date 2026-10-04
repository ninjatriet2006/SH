import type { AccountInfo } from '../../../../bridge/types';
import type { InstalledAppInfo } from '../../../../bridge/accounts_bridge';

// Platform icons
import antigravityIcon from '../../assets/icons/antigravity-menu.png';
import codebuddyIcon from '../../assets/icons/codebuddy.png';
import zedIcon from '../../assets/icons/zed.png';
import copilotIcon from '../../assets/icons/github-copilot.svg';
import cursorIcon from '../../assets/icons/cursor-menu.png';
import windsurfIcon from '../../assets/icons/windsurf.svg';
import traeIcon from '../../assets/icons/trae.png';
import claudeIcon from '../../assets/icons/claude.png';
import codexIcon from '../../assets/icons/codex.svg';
import kiroIcon from '../../assets/icons/kiro-menu.png';
import qoderIcon from '../../assets/icons/qoder.png';

export type CockpitPlatformId =
    | 'antigravity'
    | 'codebuddy'
    | 'codebuddy_cn'
    | 'codebuddy_global'
    | 'zed'
    | 'github_copilot'
    | 'cursor'
    | 'windsurf'
    | 'trae'
    | 'claude'
    | 'codex'
    | 'kiro'
    | 'qoder'
    | 'all';

export const ALL_PLATFORMS = [
    { id: 'antigravity', label: 'Antigravity', icon: antigravityIcon, path: '/platforms/antigravity' },
    { id: 'codebuddy', label: 'CodeBuddy', icon: codebuddyIcon, path: '/platforms/codebuddy' },
    { id: 'zed', label: 'Zed Cloud', icon: zedIcon, path: '/platforms/zed' },
    { id: 'github_copilot', label: 'GitHub Copilot', icon: copilotIcon, path: '/platforms/github-copilot' },
    { id: 'cursor', label: 'Cursor', icon: cursorIcon, path: '/platforms/cursor' },
    { id: 'windsurf', label: 'Windsurf', icon: windsurfIcon, path: '/platforms/windsurf' },
    { id: 'trae', label: 'Trae', icon: traeIcon, path: '/platforms/trae' },
    { id: 'claude', label: 'Claude', icon: claudeIcon, path: '/platforms/claude' },
    { id: 'codex', label: 'Codex', icon: codexIcon, path: '/platforms/codex' },
    { id: 'kiro', label: 'Kiro', icon: kiroIcon, path: '/platforms/kiro' },
    { id: 'qoder', label: 'Qoder', icon: qoderIcon, path: '/platforms/qoder' },
];

export interface CockpitAccountManagerViewProps {
    platformId: CockpitPlatformId;
    platformLabel: string;
    platformIcon: string;
    noticeTitle: string;
    permissionScope: string;
    networkScope: string;
}

export interface PlatformVariant {
    id: string;
    label: string;
    subtext: string;
    icon: string;
    isActive: boolean;
    onSelect: () => void;
}

export type { AccountInfo, InstalledAppInfo };
