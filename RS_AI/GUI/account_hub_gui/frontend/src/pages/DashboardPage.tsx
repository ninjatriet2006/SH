import React from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { Mail, Globe, CheckCircle2, ShieldAlert, Sparkles, TrendingUp } from 'lucide-react';

export const DashboardPage: React.FC = () => {
  const { data } = useAppStore();
  const { t } = useTranslation();

  if (!data) return null;

  const totalEmails = data.emails.length;
  const totalWebsites = data.websites.length;
  const registeredCount = data.registrations.filter(r => r.is_registered).length;
  const totalPossible = totalEmails * totalWebsites;
  const regPercent = totalPossible > 0 ? Math.round((registeredCount / totalPossible) * 100) : 0;

  const checkinWebsites = data.websites.filter(w => w.has_daily_checkin).length;
  const cheatWebsites = data.websites.filter(w => w.can_cheat_account).length;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div>
        <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>{t('app.title')}</h1>
        <p style={{ color: 'var(--text-secondary)' }}>{t('app.subtitle')}</p>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(220px, 1fr))', gap: '1rem' }}>
        <div className="glass-card" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(59, 130, 246, 0.2)', color: '#3b82f6' }}>
            <Mail size={24} />
          </div>
          <div>
            <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Gmail Accounts</div>
            <div style={{ fontSize: '1.6rem', fontWeight: 700 }}>{totalEmails}</div>
          </div>
        </div>

        <div className="glass-card" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(16, 185, 129, 0.2)', color: '#10b981' }}>
            <Globe size={24} />
          </div>
          <div>
            <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Websites / Platforms</div>
            <div style={{ fontSize: '1.6rem', fontWeight: 700 }}>{totalWebsites}</div>
          </div>
        </div>

        <div className="glass-card" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(168, 85, 247, 0.2)', color: '#a855f7' }}>
            <CheckCircle2 size={24} />
          </div>
          <div>
            <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Active Registrations</div>
            <div style={{ fontSize: '1.6rem', fontWeight: 700 }}>{registeredCount} <span style={{ fontSize: '0.9rem', color: 'var(--text-secondary)' }}>({regPercent}%)</span></div>
          </div>
        </div>

        <div className="glass-card" style={{ display: 'flex', alignItems: 'center', gap: '1rem' }}>
          <div style={{ padding: '0.8rem', borderRadius: '10px', background: 'rgba(245, 158, 11, 0.2)', color: '#f59e0b' }}>
            <Sparkles size={24} />
          </div>
          <div>
            <div style={{ fontSize: '0.85rem', color: 'var(--text-secondary)' }}>Daily Check-in Sites</div>
            <div style={{ fontSize: '1.6rem', fontWeight: 700 }}>{checkinWebsites}</div>
          </div>
        </div>
      </div>

      <div className="glass-card">
        <h3 style={{ marginBottom: '1rem', display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <TrendingUp size={20} color="var(--primary)" /> Tỷ lệ Đăng ký & Phủ sóng
        </h3>
        <div style={{ height: '12px', background: 'rgba(255,255,255,0.1)', borderRadius: '6px', overflow: 'hidden', marginBottom: '0.5rem' }}>
          <div style={{ height: '100%', width: `${regPercent}%`, background: 'var(--primary)', transition: 'width 0.5s ease' }} />
        </div>
        <div style={{ display: 'flex', justifyContent: 'space-between', fontSize: '0.85rem', color: 'var(--text-secondary)' }}>
          <span>{registeredCount} liên kết đã tạo</span>
          <span>{totalPossible} liên kết khả dụng</span>
        </div>
      </div>

      <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1.5rem' }}>
        <div className="glass-card">
          <h3 style={{ marginBottom: '1rem' }}>Top Websites Cần Điểm Danh</h3>
          {data.websites.filter(w => w.has_daily_checkin).map(w => (
            <div key={w.id} style={{ display: 'flex', justifyContent: 'space-between', padding: '0.6rem 0', borderBottom: '1px solid var(--border)' }}>
              <span>{w.name}</span>
              <span className="badge badge-warning">Điểm danh hàng ngày</span>
            </div>
          ))}
        </div>

        <div className="glass-card">
          <h3 style={{ marginBottom: '1rem' }}>Web Hỗ Trợ Multi-Account / Cheat</h3>
          {data.websites.filter(w => w.can_cheat_account).map(w => (
            <div key={w.id} style={{ display: 'flex', justifyContent: 'space-between', padding: '0.6rem 0', borderBottom: '1px solid var(--border)' }}>
              <span>{w.name}</span>
              <span className="badge badge-success">Cho phép tạo nhiều acc</span>
            </div>
          ))}
        </div>
      </div>
    </div>
  );
};
