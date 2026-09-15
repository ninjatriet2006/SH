import React, { useState } from 'react';
import { useAppStore } from '../store';
import { RegistrationRecord } from '../../../bridge/types';
import { Search, Globe, Mail, Sparkles, Trash2 } from 'lucide-react';
import { CheckinButton, StatusButton } from '../components/CheckinControls';

type StatusFilter = 'all' | 'live' | 'die';
type CheckinFilter = 'all' | 'checked' | 'unchecked' | 'site';

export const RegistrationsPage: React.FC = () => {
  const { data, unlinkRegistration } = useAppStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [statusFilter, setStatusFilter] = useState<StatusFilter>('all');
  const [checkinFilter, setCheckinFilter] = useState<CheckinFilter>('all');

  if (!data) return null;

  const emailMap = new Map(data.emails.map(e => [e.id, e]));
  const websiteMap = new Map(data.websites.map(w => [w.id, w]));

  // Chỉ tính các đăng ký mà email & website còn tồn tại để bộ đếm khớp đúng với bảng
  const activeRegistrations = data.registrations.filter(
    r => r.is_registered && emailMap.has(r.email_id) && websiteMap.has(r.website_id)
  );

  const liveCount = activeRegistrations.filter(r => (r.status || 'live') === 'live').length;
  const dieCount = activeRegistrations.length - liveCount;
  const onCheckinSite = (r: RegistrationRecord) => websiteMap.get(r.website_id)!.has_daily_checkin;
  const checkinSiteCount = activeRegistrations.filter(onCheckinSite).length;
  const checkedInCount = activeRegistrations.filter(r => onCheckinSite(r) && r.is_checked_in).length;
  const pendingCheckinCount = checkinSiteCount - checkedInCount;

  const filteredRegistrations = activeRegistrations.filter(r => {
    const email = emailMap.get(r.email_id)!;
    const web = websiteMap.get(r.website_id)!;

    const matchSearch =
      email.email.toLowerCase().includes(searchTerm.toLowerCase()) ||
      email.owner.toLowerCase().includes(searchTerm.toLowerCase()) ||
      web.name.toLowerCase().includes(searchTerm.toLowerCase()) ||
      web.url.toLowerCase().includes(searchTerm.toLowerCase());

    const matchStatus = statusFilter === 'all' ? true : (r.status || 'live') === statusFilter;
    const matchCheckin =
      checkinFilter === 'all'
        ? true
        : checkinFilter === 'site'
          ? web.has_daily_checkin
          : web.has_daily_checkin && (checkinFilter === 'checked' ? r.is_checked_in : !r.is_checked_in);

    return matchSearch && matchStatus && matchCheckin;
  });

  const handleUnlink = async (emailId: string, webId: string) => {
    if (window.confirm('Bạn có chắc muốn hủy liên kết này không?')) {
      await unlinkRegistration(emailId, webId);
    }
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div>
        <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>Danh Sách Đăng Ký (Live / Die Tracker)</h1>
        <p style={{ color: 'var(--text-secondary)' }}>Theo dõi tài khoản Gmail đã đăng ký web nào, lọc theo trạng thái Live/Die và tình trạng điểm danh hôm nay</p>
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

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>Trạng thái:</span>
          <select
            className="filter-select"
            value={statusFilter}
            onChange={e => setStatusFilter(e.target.value as StatusFilter)}
            title="Lọc theo trạng thái tài khoản (Live / Die)"
          >
            <option value="all">Tất cả ({activeRegistrations.length})</option>
            <option value="live">🟢 Live ({liveCount})</option>
            <option value="die">🔴 Die ({dieCount})</option>
          </select>
        </div>

        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
          <span style={{ fontSize: '0.8rem', color: 'var(--text-secondary)', whiteSpace: 'nowrap' }}>Điểm danh hôm nay:</span>
          <select
            className="filter-select"
            value={checkinFilter}
            onChange={e => setCheckinFilter(e.target.value as CheckinFilter)}
            title="Lọc theo tình trạng điểm danh hôm nay"
          >
            <option value="all">Tất cả</option>
            <option value="checked">✅ Đã điểm danh ({checkedInCount})</option>
            <option value="unchecked">⏳ Chưa điểm danh ({pendingCheckinCount})</option>
            <option value="site">Web có điểm danh ({checkinSiteCount})</option>
          </select>
        </div>
      </div>

      <div className="glass-card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-container">
          <table>
            <thead>
              <tr>
                <th>Gmail Account</th>
                <th>Website / Platform</th>
                <th>Tiêu chí Website</th>
                <th>Điểm danh & Trạng thái</th>
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
                        <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem' }}>
                          {web.has_daily_checkin && (
                            <CheckinButton
                              emailId={reg.email_id}
                              websiteId={reg.website_id}
                              checkedIn={reg.is_checked_in}
                              streak={reg.checkin_streak}
                            />
                          )}
                          <StatusButton
                            emailId={reg.email_id}
                            websiteId={reg.website_id}
                            status={reg.status}
                          />
                        </div>
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
