import React, { useState } from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { EmailAccount } from '../../../bridge/types';
import { Plus, Trash2, Edit, Mail, Tag, User, Globe2 } from 'lucide-react';
import { EmailWebsitesModal } from '../components/EmailWebsitesModal';

export const EmailsPage: React.FC = () => {
  const { data, saveEmail, deleteEmail } = useAppStore();
  const { t } = useTranslation();
  const [modalOpen, setModalOpen] = useState(false);
  const [editingEmail, setEditingEmail] = useState<EmailAccount | null>(null);
  const [selectedForWebsites, setSelectedForWebsites] = useState<EmailAccount | null>(null);

  const [formEmail, setFormEmail] = useState('');
  const [formOwner, setFormOwner] = useState('');
  const [formRecovery, setFormRecovery] = useState('');
  const [formPhone, setFormPhone] = useState('');
  const [formTags, setFormTags] = useState('');
  const [formNotes, setFormNotes] = useState('');

  if (!data) return null;

  const handleOpenAdd = () => {
    setEditingEmail(null);
    setFormEmail('');
    setFormOwner('');
    setFormRecovery('');
    setFormPhone('');
    setFormTags('');
    setFormNotes('');
    setModalOpen(true);
  };

  const handleOpenEdit = (e: EmailAccount) => {
    setEditingEmail(e);
    setFormEmail(e.email);
    setFormOwner(e.owner);
    setFormRecovery(e.recovery_email);
    setFormPhone(e.phone);
    setFormTags(e.tags.join(', '));
    setFormNotes(e.notes);
    setModalOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formEmail.trim()) return;

    const emailObj: EmailAccount = {
      id: editingEmail ? editingEmail.id : '',
      email: formEmail.trim(),
      owner: formOwner.trim(),
      recovery_email: formRecovery.trim(),
      phone: formPhone.trim(),
      tags: formTags.split(',').map(s => s.trim()).filter(Boolean),
      notes: formNotes.trim(),
      created_at: editingEmail ? editingEmail.created_at : '',
    };

    await saveEmail(emailObj);
    setModalOpen(false);
  };

  const handleDelete = async (id: string) => {
    if (window.confirm(t('common.confirm_delete'))) {
      await deleteEmail(id);
    }
  };

  const getStats = (emailId: string) => {
    const regs = data.registrations.filter(r => r.email_id === emailId && r.is_registered);
    const liveCount = regs.filter(r => (r.status || 'live') === 'live').length;
    const dieCount = regs.filter(r => r.status === 'die').length;
    return { total: regs.length, live: liveCount, die: dieCount };
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>{t('email.title')}</h1>
          <p style={{ color: 'var(--text-secondary)' }}>Danh sách tài khoản Gmail và phân loại theo dự án/chủ sở hữu</p>
        </div>
        <button className="btn btn-primary" onClick={handleOpenAdd}>
          <Plus size={18} /> {t('common.add')}
        </button>
      </div>

      <div className="glass-card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-container">
          <table>
            <thead>
              <tr>
                <th>Email</th>
                <th>Chủ sở hữu / Nhóm</th>
                <th>Khôi phục / Phone</th>
                <th>Nhãn (Tags)</th>
                <th>Web Đã Đăng Ký (Live / Die)</th>
                <th style={{ textAlign: 'right' }}>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {data.emails.map(email => {
                const stats = getStats(email.id);
                return (
                  <tr key={email.id}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                        <Mail size={16} color="var(--primary)" /> {email.email}
                      </div>
                      {email.notes && (
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '2px' }}>
                          {email.notes}
                        </div>
                      )}
                    </td>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                        <User size={14} color="var(--text-secondary)" /> {email.owner || '—'}
                      </div>
                    </td>
                    <td style={{ fontSize: '0.85rem' }}>
                      <div>{email.recovery_email || '—'}</div>
                      <div style={{ color: 'var(--text-secondary)' }}>{email.phone}</div>
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                        {email.tags.map(tag => (
                          <span key={tag} className="badge badge-gray">
                            <Tag size={10} /> {tag}
                          </span>
                        ))}
                      </div>
                    </td>
                    <td>
                      <button
                        className="btn btn-secondary"
                        style={{ padding: '0.35rem 0.75rem', fontSize: '0.8rem', gap: '0.4rem' }}
                        onClick={() => setSelectedForWebsites(email)}
                        title="Bấm để xem và thêm/hủy web cho email này"
                      >
                        <Globe2 size={14} color="var(--primary)" />
                        <span style={{ fontWeight: 600 }}>{stats.total} web</span>
                        {stats.total > 0 && (
                          <span style={{ fontSize: '0.75rem', opacity: 0.8 }}>
                            ({stats.live} Live / {stats.die} Die)
                          </span>
                        )}
                      </button>
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                        <button className="btn btn-secondary" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleOpenEdit(email)}>
                          <Edit size={14} />
                        </button>
                        <button className="btn btn-danger" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleDelete(email.id)}>
                          <Trash2 size={14} />
                        </button>
                      </div>
                    </td>
                  </tr>
                );
              })}
            </tbody>
          </table>
        </div>
      </div>

      {modalOpen && (
        <div className="modal-overlay" onClick={() => setModalOpen(false)}>
          <div className="modal-content" onClick={e => e.stopPropagation()}>
            <h2 style={{ marginBottom: '1.5rem' }}>
              {editingEmail ? 'Chỉnh sửa Gmail' : t('email.add_title')}
            </h2>
            <form onSubmit={handleSave}>
              <div className="form-group">
                <label>{t('email.email_address')} *</label>
                <input
                  type="email"
                  className="form-control"
                  required
                  value={formEmail}
                  onChange={e => setFormEmail(e.target.value)}
                  placeholder="example@gmail.com"
                />
              </div>

              <div className="form-group">
                <label>{t('email.owner')}</label>
                <input
                  type="text"
                  className="form-control"
                  value={formOwner}
                  onChange={e => setFormOwner(e.target.value)}
                  placeholder="Team Main, Dev, Airdrop..."
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                <div className="form-group">
                  <label>{t('email.recovery_email')}</label>
                  <input
                    type="email"
                    className="form-control"
                    value={formRecovery}
                    onChange={e => setFormRecovery(e.target.value)}
                    placeholder="backup@gmail.com"
                  />
                </div>
                <div className="form-group">
                  <label>{t('email.phone')}</label>
                  <input
                    type="text"
                    className="form-control"
                    value={formPhone}
                    onChange={e => setFormPhone(e.target.value)}
                    placeholder="0987654321"
                  />
                </div>
              </div>

              <div className="form-group">
                <label>{t('email.tags')}</label>
                <input
                  type="text"
                  className="form-control"
                  value={formTags}
                  onChange={e => setFormTags(e.target.value)}
                  placeholder="Main, KYC, Testnet, Bot"
                />
              </div>

              <div className="form-group">
                <label>{t('common.notes')}</label>
                <textarea
                  className="form-control"
                  rows={3}
                  value={formNotes}
                  onChange={e => setFormNotes(e.target.value)}
                  placeholder="Ghi chú về tài khoản này..."
                />
              </div>

              <div style={{ display: 'flex', gap: '1rem', justifyContent: 'flex-end', marginTop: '1.5rem' }}>
                <button type="button" className="btn btn-secondary" onClick={() => setModalOpen(false)}>
                  {t('common.cancel')}
                </button>
                <button type="submit" className="btn btn-primary">
                  {t('common.save')}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {selectedForWebsites && (
        <EmailWebsitesModal
          email={selectedForWebsites}
          onClose={() => setSelectedForWebsites(null)}
        />
      )}
    </div>
  );
};
