import React, { useState } from 'react';
import { useAppStore } from '../store';
import { EmailAccount, Website, RegistrationRecord } from '../../../bridge/types';
import { Globe, Plus, Trash2, Search, ExternalLink, Sparkles, CheckSquare, ShieldAlert, Check } from 'lucide-react';

interface EmailWebsitesModalProps {
  email: EmailAccount;
  onClose: () => void;
}

export const EmailWebsitesModal: React.FC<EmailWebsitesModalProps> = ({ email, onClose }) => {
  const { data, toggleRegistration, setRegistrationStatus, unlinkRegistration } = useAppStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [showAddSection, setShowAddSection] = useState(false);
  const [filterCheckinOnly, setFilterCheckinOnly] = useState(false);
  const [filterCheatOnly, setFilterCheatOnly] = useState(false);

  if (!data) return null;

  // Lấy các đăng ký của email này
  const emailRegistrations = data.registrations.filter(r => r.email_id === email.id && r.is_registered);
  const registeredWebIds = new Set(emailRegistrations.map(r => r.website_id));

  // Danh sách web đã đăng ký
  const registeredWebsites = data.websites.filter(w => registeredWebIds.has(w.id));

  // Danh sách web CHƯA đăng ký (để thêm nhanh)
  const unregisteredWebsites = data.websites.filter(w => {
    if (registeredWebIds.has(w.id)) return false;
    const matchSearch = w.name.toLowerCase().includes(searchTerm.toLowerCase()) || w.url.toLowerCase().includes(searchTerm.toLowerCase());
    const matchCheckin = filterCheckinOnly ? w.has_daily_checkin : true;
    const matchCheat = filterCheatOnly ? w.can_cheat_account : true;
    return matchSearch && matchCheckin && matchCheat;
  });

  const getRecord = (webId: string): RegistrationRecord | undefined => {
    return emailRegistrations.find(r => r.website_id === webId);
  };

  const handleToggleStatus = async (webId: string, currentStatus: string) => {
    const nextStatus = currentStatus === 'live' ? 'die' : 'live';
    await setRegistrationStatus(email.id, webId, nextStatus as any);
  };

  const handleLinkWeb = async (webId: string) => {
    await toggleRegistration(email.id, webId);
  };

  const handleUnlink = async (webId: string) => {
    if (window.confirm('Hủy liên kết tài khoản này khỏi website?')) {
      await unlinkRegistration(email.id, webId);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '800px', width: '95%' }} onClick={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem', borderBottom: '1px solid var(--border)', paddingBottom: '0.8rem' }}>
          <div>
            <h2 style={{ fontSize: '1.3rem', marginBottom: '0.2rem' }}>Quản lý Web của Email</h2>
            <div style={{ color: 'var(--primary)', fontWeight: 600, fontSize: '0.95rem' }}>{email.email} ({email.owner || 'Chưa phân nhóm'})</div>
          </div>
          <button className="btn btn-primary" onClick={() => setShowAddSection(!showAddSection)}>
            <Plus size={16} /> {showAddSection ? 'Đóng bộ chọn' : '+ Đăng ký thêm Web'}
          </button>
        </div>

        {/* Khu vực Đăng ký thêm Website mới (Search & Select) */}
        {showAddSection && (
          <div style={{ background: 'rgba(15, 23, 42, 0.7)', border: '1px solid var(--border)', borderRadius: '8px', padding: '1rem', marginBottom: '1.5rem' }}>
            <h4 style={{ marginBottom: '0.8rem', fontSize: '0.95rem' }}>Chọn Website để liên kết / đánh dấu đăng ký</h4>
            <div style={{ display: 'flex', gap: '0.8rem', marginBottom: '0.8rem', flexWrap: 'wrap' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'rgba(0,0,0,0.4)', padding: '0.4rem 0.8rem', borderRadius: '6px', border: '1px solid var(--border)', flex: 1, minWidth: '200px' }}>
                <Search size={14} color="var(--text-secondary)" />
                <input
                  type="text"
                  placeholder="Tìm website theo tên, domain..."
                  value={searchTerm}
                  onChange={e => setSearchTerm(e.target.value)}
                  style={{ background: 'transparent', border: 'none', color: '#fff', outline: 'none', width: '100%', fontSize: '0.85rem' }}
                />
              </div>

              <label className="checkbox-label" style={{ fontSize: '0.8rem' }}>
                <input
                  type="checkbox"
                  checked={filterCheckinOnly}
                  onChange={e => setFilterCheckinOnly(e.target.checked)}
                />
                <span>Có Điểm danh</span>
              </label>

              <label className="checkbox-label" style={{ fontSize: '0.8rem' }}>
                <input
                  type="checkbox"
                  checked={filterCheatOnly}
                  onChange={e => setFilterCheatOnly(e.target.checked)}
                />
                <span>Hỗ trợ Cheat</span>
              </label>
            </div>

            <div style={{ maxHeight: '200px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
              {unregisteredWebsites.length === 0 ? (
                <div style={{ textAlign: 'center', padding: '1rem', color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                  Không có website phù hợp hoặc đã đăng ký tất cả
                </div>
              ) : (
                unregisteredWebsites.map(w => (
                  <div key={w.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.03)', padding: '0.5rem 0.8rem', borderRadius: '6px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                      <Globe size={16} color="var(--primary)" />
                      <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>{w.name}</span>
                      <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>{w.url}</span>
                      {w.has_daily_checkin && <span className="badge badge-warning" style={{ fontSize: '10px' }}>Điểm danh</span>}
                      {w.can_cheat_account && <span className="badge badge-success" style={{ fontSize: '10px' }}>Cheat</span>}
                    </div>
                    <button className="btn btn-secondary" style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem' }} onClick={() => handleLinkWeb(w.id)}>
                      <Check size={14} /> Gán đăng ký
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* Danh sách Website đã đăng ký */}
        <h3 style={{ fontSize: '1.05rem', marginBottom: '0.8rem' }}>
          Danh sách Web Đã Đăng Ký ({registeredWebsites.length})
        </h3>

        {registeredWebsites.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)', background: 'rgba(0,0,0,0.2)', borderRadius: '8px' }}>
            Email này chưa đăng ký web nào. Hãy bấm "+ Đăng ký thêm Web" ở trên.
          </div>
        ) : (
          <div style={{ maxHeight: '350px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
            {registeredWebsites.map(web => {
              const rec = getRecord(web.id);
              const isLive = (rec?.status || 'live') === 'live';
              return (
                <div key={web.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.04)', padding: '0.75rem 1rem', borderRadius: '8px', border: '1px solid var(--border)' }}>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                      <Globe size={16} color="var(--primary)" />
                      <span>{web.name}</span>
                      {web.url && (
                        <a href={web.url} target="_blank" rel="noreferrer" style={{ color: 'var(--text-secondary)' }}>
                          <ExternalLink size={12} />
                        </a>
                      )}
                    </div>
                    <div style={{ display: 'flex', gap: '6px', marginTop: '4px', flexWrap: 'wrap' }}>
                      <span className="badge badge-gray">{web.category || 'General'}</span>
                      {web.has_daily_checkin && <span className="badge badge-warning" style={{ fontSize: '10px' }}><Sparkles size={10} /> Điểm danh</span>}
                      {web.can_cheat_account && <span className="badge badge-success" style={{ fontSize: '10px' }}><CheckSquare size={10} /> Cheat Acc</span>}
                      {web.requires_kyc && <span className="badge badge-gray" style={{ color: '#ec4899', fontSize: '10px' }}>KYC</span>}
                      {web.requires_proxy && <span className="badge badge-gray" style={{ color: '#38bdf8', fontSize: '10px' }}>Proxy</span>}
                    </div>
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.8rem' }}>
                    {/* Trạng thái tài khoản Live / Die */}
                    <button
                      type="button"
                      className="btn"
                      onClick={() => handleToggleStatus(web.id, rec?.status || 'live')}
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

                    {/* Nút hủy liên kết */}
                    <button
                      type="button"
                      className="btn btn-danger"
                      style={{ padding: '0.35rem 0.6rem' }}
                      onClick={() => handleUnlink(web.id)}
                      title="Hủy đăng ký website này"
                    >
                      <Trash2 size={14} />
                    </button>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        <div style={{ display: 'flex', justifyContent: 'flex-end', marginTop: '1.5rem', borderTop: '1px solid var(--border)', paddingTop: '1rem' }}>
          <button className="btn btn-secondary" onClick={onClose}>
            Đóng
          </button>
        </div>
      </div>
    </div>
  );
};
