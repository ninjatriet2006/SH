import React, { useState } from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { Search, Globe, Mail, CheckCircle2, ShieldAlert, Sparkles, Filter, Trash2 } from 'lucide-react';

export const RegistrationsPage: React.FC = () => {
  const { data, setRegistrationStatus, unlinkRegistration } = useAppStore();
  const { t } = useTranslation();
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<'all' | 'live' | 'die'>('all');
  const [filterCheckinOnly, setFilterCheckinOnly] = useState(false);

  if (!data) return null;

  const emailMap = new Map(data.emails.map(e => [e.id, e]));
  const websiteMap = new Map(data.websites.map(w => [w.id, w]));

  const activeRegistrations = data.registrations.filter(r => r.is_registered);

  const filteredRegistrations = activeRegistrations.filter(r => {
    const email = emailMap.get(r.email_id);
    const web = websiteMap.get(r.website_id);
    if (!email || !web) return false;

    const matchSearch =
      email.email.toLowerCase().includes(searchTerm.toLowerCase()) ||
      email.owner.toLowerCase().includes(searchTerm.toLowerCase()) ||
      web.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      web.url.toLowerCase().includes(searchTerm.toLowerCase());

    const recStatus = r.status || 'live';
    const matchStatus = statusFilter === 'all' ? true : recStatus === statusFilter;
    const matchCheckin = filterCheckinOnly ? web.has_daily_checkin : true;

    return matchSearch && matchStatus && matchCheckin;
  });

  const handleToggleStatus = async (emailId: string, webId: string, currentStatus: string) => {
    const nextStatus = currentStatus === 'live' ? 'die' : 'live';
    await setRegistrationStatus(emailId, webId, nextStatus as any);
  };

  const handleUnlink = async (emailId: string, webId: string) => {
    if (window.confirm('Bạn có chắc muốn hủy liên kết này không?')) {
      await unlinkRegistration(emailId, webId);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div>
        <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>Danh Sách Đăng Ký (Live / Die Tracker)</h1>
        <p style={{ color: 'var(--text-secondary)' }}>Theo dõi chi tiết các tài khoản Gmail đã đăng ký web nào, lọc nhanh theo trạng thái Live/Die</p>
      </div>

      <div className="glass-card" style={{ display: 'flex', gap: '1rem', flexWrap: 'wrap', alignItems: 'center' }}>
        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'rgba(15,23,42,0.8)', padding: '0.4rem 0.8rem', borderRadius: '6px', border: '1px solid var(--border)', flex: 1, minWidth: '220px' }}>
          <Search size={16} color="var(--text-secondary)" />
          <input
            type="text"
            placeholder="Tìm theo Gmail, nhóm hoặc Tên Web..."
            value={searchTerm}
            onChange={e => setSearchTerm(e.target.value)}
            style={{ background: 'transparent', border: 'none', color: '#fff', outline: 'none', width: '100%' }}
          />
        </div>

        <div style={{ display: 'flex', gap: '0.5rem' }}>
          <button
            className={`btn ${statusFilter === 'all' ? 'btn-primary' : 'btn-secondary'}`}
            style={{ padding: '0.4rem 0.8rem', fontSize: '0.85rem' }}
            onClick={() => setStatusFilter('all')}
          >
            Tất cả ({activeRegistrations.length})
          </button>
          <button
            className={`btn ${statusFilter === 'live' ? 'btn-primary' : 'btn-secondary'}`}
            style={{ padding: '0.4rem 0.8rem', fontSize: '0.85rem' }}
            onClick={() => setStatusFilter('live')}
          >
            🟢 Live ({activeRegistrations.filter(r => (r.status || 'live') === 'live').length})
          </button>
          <button
            className={`btn ${statusFilter === 'die' ? 'btn-primary' : 'btn-secondary'}`}
            style={{ padding: '0.4rem 0.8rem', fontSize: '0.85rem' }}
            onClick={() => setStatusFilter('die')}
          >
            🔴 Die ({activeRegistrations.filter(r => r.status === 'die').length})
          </button>
        </div>

        <label className="checkbox-label" style={{ background: 'rgba(255,255,255,0.05)', padding: '0.5rem 0.8rem', borderRadius: '6px' }}>
          <input
            type="checkbox"
            checked={filterCheckinOnly}
            onChange={e => setFilterCheckinOnly(e.target.checked)}
          />
          <span>Web có điểm danh</span>
        </label>
      </div>

      <div className="glass-card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-container">
          <table>
            <thead>
              <tr>
                <th>Gmail Account</th>
                <th>Website / Platform</th>
                <th>Tiêu chí Website</th>
                <th>Trạng thái Acc</th>
                <th style={{ textAlign: 'right' }}>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {filteredRegistrations.length === 0 ? (
                <tr>
                  <td colSpan={5} style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)' }}>
                    Không tìm thấy bản ghi đăng ký nào phù hợp
                  </td>
                </tr>
              ) : (
                filteredRegistrations.map(reg => {
                  const email = emailMap.get(reg.email_id);
                  const web = websiteMap.get(reg.website_id);
                  if (!email || !web) return null;
                  const isLive = (reg.status || 'live') === 'live';

                  return (
                    <tr key={reg.id}>
                      <td>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                          <Mail size={16} color="var(--primary)" />
                          <span>{email.email}</span>
                        </div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                          {email.owner || 'Chưa phân nhóm'}
                        </div>
                      </td>
                      <td>
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                          <Globe size={16} color="#10b981" />
                          <span>{web.name}</span>
                        </div>
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>
                          {web.url || 'No URL'}
                        </div>
                      </td>
                      <td>
                        <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                          {web.has_daily_checkin && <span className="badge badge-warning" style={{ fontSize: '10px' }}><Sparkles size={10} /> Điểm danh</span>}
                          {web.can_cheat_account && <span className="badge badge-success" style={{ fontSize: '10px' }}>Cheat</span>}
                          {web.requires_kyc && <span className="badge badge-gray" style={{ color: '#ec4899', fontSize: '10px' }}>KYC</span>}
                          {web.requires_proxy && <span className="badge badge-gray" style={{ color: '#38bdf8', fontSize: '10px' }}>Proxy</span>}
                        </div>
                      </td>
                      <td>
                        <button
                          type="button"
                          className="btn"
                          onClick={() => handleToggleStatus(reg.email_id, reg.website_id, reg.status || 'live')}
                          style={{
                            padding: '0.35rem 0.8rem',
                            fontSize: '0.8rem',
                            background: isLive ? 'rgba(16, 185, 129, 0.2)' : 'rgba(239, 68, 68, 0.2)',
                            color: isLive ? '#10b981' : '#ef4444',
                            border: `1px solid ${isLive ? 'rgba(16, 185, 129, 0.4)' : 'rgba(239, 68, 68, 0.4)'}`
                          }}
                          title="Click để chuyển đổi Live / Die"
                        >
                          {isLive ? '🟢 LIVE' : '🔴 DIE'}
                        </button>
                      </td>
                      <td style={{ textAlign: 'right' }}>
                        <button
                          type="button"
                          className="btn btn-danger"
                          style={{ padding: '0.4rem 0.6rem' }}
                          onClick={() => handleUnlink(reg.email_id, reg.website_id)}
                          title="Hủy liên kết đăng ký này"
                        >
                          <Trash2 size={14} />
                        </button>
                      </td>
                    </tr>
                  );
                })
              )}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
};
