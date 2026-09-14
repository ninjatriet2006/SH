import React, { useState } from 'react';
import { useAppStore } from '../store';
import { Website, EmailAccount, RegistrationRecord } from '../../../bridge/types';
import { Mail, Plus, Trash2, Search, Check, ExternalLink } from 'lucide-react';

interface WebsiteEmailsModalProps {
  website: Website;
  onClose: () => void;
}

export const WebsiteEmailsModal: React.FC<WebsiteEmailsModalProps> = ({ website, onClose }) => {
  const { data, toggleRegistration, setRegistrationStatus, unlinkRegistration } = useAppStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [showAddSection, setShowAddSection] = useState(false);

  if (!data) return null;

  // Lấy các đăng ký thuộc website này
  const siteRegistrations = data.registrations.filter(r => r.website_id === website.id && r.is_registered);
  const registeredEmailIds = new Set(siteRegistrations.map(r => r.email_id));

  // Danh sách Email đã đăng ký web này
  const registeredEmails = data.emails.filter(e => registeredEmailIds.has(e.id));

  // Danh sách Email CHƯA đăng ký web này
  const unregisteredEmails = data.emails.filter(e => {
    if (registeredEmailIds.has(e.id)) return false;
    return e.email.toLowerCase().includes(searchTerm.toLowerCase()) || e.owner.toLowerCase().includes(searchTerm.toLowerCase());
  });

  const getRecord = (emailId: string): RegistrationRecord | undefined => {
    return siteRegistrations.find(r => r.email_id === emailId);
  };

  const handleToggleStatus = async (emailId: string, currentStatus: string) => {
    const nextStatus = currentStatus === 'live' ? 'die' : 'live';
    await setRegistrationStatus(emailId, website.id, nextStatus as any);
  };

  const handleLinkEmail = async (emailId: string) => {
    await toggleRegistration(emailId, website.id);
  };

  const handleUnlink = async (emailId: string) => {
    if (window.confirm('Hủy liên kết tài khoản này khỏi website?')) {
      await unlinkRegistration(emailId, website.id);
    }
  };

  return (
    <div className="modal-overlay" onClick={onClose}>
      <div className="modal-content" style={{ maxWidth: '800px', width: '95%' }} onClick={e => e.stopPropagation()}>
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem', borderBottom: '1px solid var(--border)', paddingBottom: '0.8rem' }}>
          <div>
            <h2 style={{ fontSize: '1.3rem', marginBottom: '0.2rem' }}>Gmail Đã Đăng Ký</h2>
            <div style={{ color: 'var(--primary)', fontWeight: 600, fontSize: '0.95rem' }}>{website.name} ({website.url || 'No URL'})</div>
          </div>
          <button className="btn btn-primary" onClick={() => setShowAddSection(!showAddSection)}>
            <Plus size={16} /> {showAddSection ? 'Đóng bộ chọn' : '+ Gán thêm Gmail'}
          </button>
        </div>

        {/* Bộ chọn thêm Gmail mới */}
        {showAddSection && (
          <div style={{ background: 'rgba(15, 23, 42, 0.7)', border: '1px solid var(--border)', borderRadius: '8px', padding: '1rem', marginBottom: '1.5rem' }}>
            <h4 style={{ marginBottom: '0.8rem', fontSize: '0.95rem' }}>Chọn Gmail để đánh dấu đã đăng ký {website.name}</h4>
            <div style={{ display: 'flex', gap: '0.8rem', marginBottom: '0.8rem' }}>
              <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', background: 'rgba(0,0,0,0.4)', padding: '0.4rem 0.8rem', borderRadius: '6px', border: '1px solid var(--border)', flex: 1 }}>
                <Search size={14} color="var(--text-secondary)" />
                <input
                  type="text"
                  placeholder="Tìm theo email hoặc nhóm/chủ sở hữu..."
                  value={searchTerm}
                  onChange={e => setSearchTerm(e.target.value)}
                  style={{ background: 'transparent', border: 'none', color: '#fff', outline: 'none', width: '100%', fontSize: '0.85rem' }}
                />
              </div>
            </div>

            <div style={{ maxHeight: '200px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
              {unregisteredEmails.length === 0 ? (
                <div style={{ textAlign: 'center', padding: '1rem', color: 'var(--text-secondary)', fontSize: '0.85rem' }}>
                  Tất cả các Gmail hiện có đã được đăng ký web này
                </div>
              ) : (
                unregisteredEmails.map(e => (
                  <div key={e.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.03)', padding: '0.5rem 0.8rem', borderRadius: '6px' }}>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem' }}>
                      <Mail size={16} color="var(--primary)" />
                      <span style={{ fontWeight: 600, fontSize: '0.9rem' }}>{e.email}</span>
                      <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>({e.owner || 'Chưa nhóm'})</span>
                    </div>
                    <button className="btn btn-secondary" style={{ padding: '0.3rem 0.6rem', fontSize: '0.8rem' }} onClick={() => handleLinkEmail(e.id)}>
                      <Check size={14} /> Gán đăng ký
                    </button>
                  </div>
                ))
              )}
            </div>
          </div>
        )}

        {/* Danh sách Email đã đăng ký */}
        <h3 style={{ fontSize: '1.05rem', marginBottom: '0.8rem' }}>
          Danh sách Gmail Đã Đăng Ký ({registeredEmails.length})
        </h3>

        {registeredEmails.length === 0 ? (
          <div style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)', background: 'rgba(0,0,0,0.2)', borderRadius: '8px' }}>
            Chưa có tài khoản Gmail nào đăng ký web này. Hãy bấm "+ Gán thêm Gmail" ở trên.
          </div>
        ) : (
          <div style={{ maxHeight: '350px', overflowY: 'auto', display: 'flex', flexDirection: 'column', gap: '0.6rem' }}>
            {registeredEmails.map(e => {
              const rec = getRecord(e.id);
              const isLive = (rec?.status || 'live') === 'live';
              return (
                <div key={e.id} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.04)', padding: '0.75rem 1rem', borderRadius: '8px', border: '1px solid var(--border)' }}>
                  <div>
                    <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                      <Mail size={16} color="var(--primary)" />
                      <span>{e.email}</span>
                      <span style={{ fontSize: '0.75rem', color: 'var(--text-secondary)' }}>• {e.owner || 'Chưa nhóm'}</span>
                    </div>
                    {e.notes && (
                      <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '2px' }}>
                        {e.notes}
                      </div>
                    )}
                  </div>

                  <div style={{ display: 'flex', alignItems: 'center', gap: '0.8rem' }}>
                    <button
                      type="button"
                      className="btn"
                      onClick={() => handleToggleStatus(e.id, rec?.status || 'live')}
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

                    <button
                      type="button"
                      className="btn btn-danger"
                      style={{ padding: '0.35rem 0.6rem' }}
                      onClick={() => handleUnlink(e.id)}
                      title="Hủy liên kết Gmail này khỏi website"
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
