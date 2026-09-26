/*
[INTEGRITY NOTES]
- Mục đích: Thanh điều hướng bên trái (Sidebar) của ứng dụng.
- Trách nhiệm: Chuyển hướng các trang (Explorer, Remotes, Mounts, Transfers, Trash, Settings) và hiển thị số lượng job đang chạy.
- Tương tác: Dùng `react-router-dom` NavLink và `useJobsStore`.
*/

import {
  ArrowLeftRight,
  Cloud,
  FolderSync,
  HardDrive,
  LayoutGrid,
  Settings,
  Trash2,
} from 'lucide-react';
import React from 'react';
import { NavLink } from 'react-router-dom';
import { useJobsStore } from '../store/useJobsStore';
import { useTranslation } from '../utils/i18n';

export const Sidebar: React.FC = () => {
  const { t } = useTranslation();
  const jobs = useJobsStore((state) => state.jobs);
  const runningJobsCount = jobs.filter((j) => j.status === 'running' || j.status === 'queued').length;

  return (
    <aside className="sidebar">
      <div className="sidebar-brand">
        <FolderSync size={24} color="#818cf8" />
        <h2>Rclone GUI</h2>
      </div>

      <nav className="sidebar-nav">
        <NavLink
          to="/"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
          end
          data-lang-id="nav_explorer"
        >
          <LayoutGrid size={18} />
          <span>{t('nav_explorer', 'Trình duyệt file')}</span>
        </NavLink>

        <NavLink
          to="/remotes"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
          data-lang-id="nav_remotes"
        >
          <Cloud size={18} />
          <span>{t('nav_remotes', 'Remote đám mây')}</span>
        </NavLink>

        <NavLink
          to="/mounts"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
          data-lang-id="nav_mounts"
        >
          <HardDrive size={18} />
          <span>{t('nav_mounts', 'Dịch vụ Mount')}</span>
        </NavLink>

        <NavLink
          to="/transfers"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <ArrowLeftRight size={18} />
          <span>{t('sidebar.transfers', 'Hàng đợi tiến trình')}</span>
          {runningJobsCount > 0 && <span className="nav-badge">{runningJobsCount}</span>}
        </NavLink>

        <NavLink
          to="/trash"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
        >
          <Trash2 size={18} />
          <span>{t('sidebar.trash', 'Thùng rác')}</span>
        </NavLink>

        <NavLink
          to="/settings"
          className={({ isActive }) => `nav-link ${isActive ? 'active' : ''}`}
          data-lang-id="nav_settings"
        >
          <Settings size={18} />
          <span>{t('nav_settings', 'Cài đặt & Chẩn đoán')}</span>
        </NavLink>
      </nav>
    </aside>
  );
};
