import React from 'react';
import { useAppStore } from '../store';
import { AccountStatus } from '../../../bridge/types';
import { Sparkles } from 'lucide-react';

interface CheckinButtonProps {
  emailId: string;
  websiteId: string;
  checkedIn: boolean;
  streak: number;
}

export const CheckinButton: React.FC<CheckinButtonProps> = ({ emailId, websiteId, checkedIn, streak }) => {
  const toggleCheckin = useAppStore(state => state.toggleCheckin);

  return (
    <button
      type="button"
      className="btn"
      onClick={() => toggleCheckin(emailId, websiteId)}
      style={{
        padding: '0.35rem 0.75rem',
        fontSize: '0.8rem',
        background: checkedIn ? 'rgba(245, 158, 11, 0.25)' : 'rgba(255, 255, 255, 0.06)',
        color: checkedIn ? '#f59e0b' : 'var(--text-secondary)',
        border: `1px solid ${checkedIn ? 'rgba(245, 158, 11, 0.5)' : 'var(--border)'}`
      }}
      title={checkedIn ? `Đã điểm danh hôm nay (Streak: ${streak}). Bấm để hủy.` : 'Chưa điểm danh hôm nay. Bấm để đánh dấu đã điểm danh!'}
    >
      <Sparkles size={13} />
      {checkedIn ? `Đã điểm danh (${streak})` : 'Chưa điểm danh'}
    </button>
  );
};

interface StatusButtonProps {
  emailId: string;
  websiteId: string;
  status?: AccountStatus;
}

export const StatusButton: React.FC<StatusButtonProps> = ({ emailId, websiteId, status }) => {
  const setRegistrationStatus = useAppStore(state => state.setRegistrationStatus);
  const isLive = (status || 'live') === 'live';

  return (
    <button
      type="button"
      className="btn"
      onClick={() => setRegistrationStatus(emailId, websiteId, isLive ? 'die' : 'live')}
      style={{
        padding: '0.35rem 0.8rem',
        fontSize: '0.8rem',
        background: isLive ? 'rgba(16, 185, 129, 0.2)' : 'rgba(239, 68, 68, 0.2)',
        color: isLive ? '#10b981' : '#ef4444',
        border: `1px solid ${isLive ? 'rgba(16, 185, 129, 0.4)' : 'rgba(239, 68, 68, 0.4)'}`
      }}
      title="Click để chuyển trạng thái Live <-> Die"
    >
      {isLive ? '🟢 LIVE' : '🔴 DIE'}
    </button>
  );
};
