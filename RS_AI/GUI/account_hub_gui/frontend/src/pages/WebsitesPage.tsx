import React, { useState } from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { Website, CustomCriterion } from '../../../bridge/types';
import { Plus, Trash2, Edit, Globe, CheckSquare, Sparkles, ExternalLink, Sliders, Users } from 'lucide-react';
import { WebsiteEmailsModal } from '../components/WebsiteEmailsModal';

export const WebsitesPage: React.FC = () => {
  const { data, saveWebsite, deleteWebsite } = useAppStore();
  const { t } = useTranslation();
  const [modalOpen, setModalOpen] = useState(false);
  const [editingWeb, setEditingWeb] = useState<Website | null>(null);
  const [selectedForEmails, setSelectedForEmails] = useState<Website | null>(null);

  const [formName, setFormName] = useState('');
  const [formUrl, setFormUrl] = useState('');
  const [formCategory, setFormCategory] = useState('');
  const [formDailyCheckin, setFormDailyCheckin] = useState(false);
  const [formCheatAccount, setFormCheatAccount] = useState(false);
  const [formRequiresKyc, setFormRequiresKyc] = useState(false);
  const [formRequiresProxy, setFormRequiresProxy] = useState(false);
  const [formCustomCriteria, setFormCustomCriteria] = useState<CustomCriterion[]>([]);
  const [formNotes, setFormNotes] = useState('');

  // Fields for adding new custom criterion
  const [newCritKey, setNewCritKey] = useState('');
  const [newCritLabel, setNewCritLabel] = useState('');
  const [newCritValue, setNewCritValue] = useState('');

  if (!data) return null;

  const handleOpenAdd = () => {
    setEditingWeb(null);
    setFormName('');
    setFormUrl('');
    setFormCategory('');
    setFormDailyCheckin(false);
    setFormCheatAccount(false);
    setFormRequiresKyc(false);
    setFormRequiresProxy(false);
    setFormCustomCriteria([]);
    setFormNotes('');
    setModalOpen(true);
  };

  const handleOpenEdit = (w: Website) => {
    setEditingWeb(w);
    setFormName(w.name);
    setFormUrl(w.url);
    setFormCategory(w.category);
    setFormDailyCheckin(w.has_daily_checkin);
    setFormCheatAccount(w.can_cheat_account);
    setFormRequiresKyc(w.requires_kyc);
    setFormRequiresProxy(w.requires_proxy);
    setFormCustomCriteria([...w.custom_criteria]);
    setFormNotes(w.notes);
    setModalOpen(true);
  };

  const handleAddCriterion = () => {
    if (!newCritLabel.trim()) return;
    const key = newCritKey.trim() || newCritLabel.toLowerCase().replace(/\s+/g, '_');
    setFormCustomCriteria([
      ...formCustomCriteria,
      {
        key,
        label: newCritLabel.trim(),
        value_type: 'text',
        value: newCritValue.trim() || 'Có',
      }
    ]);
    setNewCritKey('');
    setNewCritLabel('');
    setNewCritValue('');
  };

  const handleRemoveCriterion = (index: number) => {
    setFormCustomCriteria(formCustomCriteria.filter((_, i) => i !== index));
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) return;

    const webObj: Website = {
      id: editingWeb ? editingWeb.id : '',
      name: formName.trim(),
      url: formUrl.trim(),
      category: formCategory.trim(),
      has_daily_checkin: formDailyCheckin,
      can_cheat_account: formCheatAccount,
      requires_kyc: formRequiresKyc,
      requires_proxy: formRequiresProxy,
      custom_criteria: formCustomCriteria,
      notes: formNotes.trim(),
      created_at: editingWeb ? editingWeb.created_at : '',
    };

    await saveWebsite(webObj);
    setModalOpen(false);
  };

  const handleDelete = async (id: string) => {
    if (window.confirm(t('common.confirm_delete'))) {
      await deleteWebsite(id);
    }
  };

  const getStats = (webId: string) => {
    const regs = data.registrations.filter(r => r.website_id === webId && r.is_registered);
    const liveCount = regs.filter(r => (r.status || 'live') === 'live').length;
    const dieCount = regs.filter(r => r.status === 'die').length;
    return { total: regs.length, live: liveCount, die: dieCount };
  };

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>{t('website.title')}</h1>
          <p style={{ color: 'var(--text-secondary)' }}>Danh sách website/nền tảng và các tiêu chuẩn đặc thù (điểm danh, cheat account, KYC,...)</p>
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
                <th>Website & URL</th>
                <th>Phân loại</th>
                <th>Tiêu chí đặc thù</th>
                <th>Gmail Đã Gán (Live / Die)</th>
                <th>Tiêu chí tùy chỉnh</th>
                <th style={{ textAlign: 'right' }}>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {data.websites.map(web => {
                const stats = getStats(web.id);
                return (
                  <tr key={web.id}>
                    <td>
                      <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', fontWeight: 600 }}>
                        <Globe size={16} color="var(--primary)" /> {web.name}
                        {web.url && (
                          <a href={web.url} target="_blank" rel="noreferrer" style={{ color: 'var(--text-secondary)' }}>
                            <ExternalLink size={12} />
                          </a>
                        )}
                      </div>
                      {web.notes && (
                        <div style={{ fontSize: '0.75rem', color: 'var(--text-secondary)', marginTop: '2px' }}>
                          {web.notes}
                        </div>
                      )}
                    </td>
                    <td>
                      <span className="badge badge-gray">{web.category || 'Chung'}</span>
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: '6px', flexWrap: 'wrap' }}>
                        {web.has_daily_checkin && (
                          <span className="badge badge-warning" title="Điểm danh nhận thưởng hàng ngày">
                            <Sparkles size={12} /> Điểm danh
                          </span>
                        )}
                        {web.can_cheat_account && (
                          <span className="badge badge-success" title="Có thể cheat multi-account">
                            <CheckSquare size={12} /> Cheat Acc
                          </span>
                        )}
                        {web.requires_kyc && (
                          <span className="badge badge-gray" style={{ color: '#ec4899', borderColor: 'rgba(236,72,153,0.3)' }}>
                            KYC
                          </span>
                        )}
                        {web.requires_proxy && (
                          <span className="badge badge-gray" style={{ color: '#38bdf8', borderColor: 'rgba(56,189,248,0.3)' }}>
                            Proxy
                          </span>
                        )}
                      </div>
                    </td>
                    <td>
                      <button
                        className="btn btn-secondary"
                        style={{ padding: '0.35rem 0.75rem', fontSize: '0.8rem', gap: '0.4rem' }}
                        onClick={() => setSelectedForEmails(web)}
                        title="Bấm để xem danh sách Gmail đã đăng ký và gán thêm Gmail"
                      >
                        <Users size={14} color="var(--primary)" />
                        <span style={{ fontWeight: 600 }}>{stats.total} accounts</span>
                        {stats.total > 0 && (
                          <span style={{ fontSize: '0.75rem', opacity: 0.8 }}>
                            ({stats.live} Live / {stats.die} Die)
                          </span>
                        )}
                      </button>
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                        {web.custom_criteria.map((c, idx) => (
                          <span key={idx} className="badge badge-gray" style={{ fontSize: '0.7rem' }}>
                            <strong>{c.label}:</strong> {c.value}
                          </span>
                        ))}
                        {web.custom_criteria.length === 0 && <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>—</span>}
                      </div>
                    </td>
                    <td style={{ textAlign: 'right' }}>
                      <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                        <button className="btn btn-secondary" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleOpenEdit(web)}>
                          <Edit size={14} />
                        </button>
                        <button className="btn btn-danger" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleDelete(web.id)}>
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
              {editingWeb ? 'Chỉnh sửa Website' : t('website.add_title')}
            </h2>
            <form onSubmit={handleSave}>
              <div className="form-group">
                <label>{t('website.name')} *</label>
                <input
                  type="text"
                  className="form-control"
                  required
                  value={formName}
                  onChange={e => setFormName(e.target.value)}
                  placeholder="Ví dụ: Binance, Grass, Discord..."
                />
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '1rem' }}>
                <div className="form-group">
                  <label>{t('website.url')}</label>
                  <input
                    type="url"
                    className="form-control"
                    value={formUrl}
                    onChange={e => setFormUrl(e.target.value)}
                    placeholder="https://example.com"
                  />
                </div>
                <div className="form-group">
                  <label>{t('website.category')}</label>
                  <input
                    type="text"
                    className="form-control"
                    value={formCategory}
                    onChange={e => setFormCategory(e.target.value)}
                    placeholder="Crypto, Social, AI, Game..."
                  />
                </div>
              </div>

              <div style={{ display: 'grid', gridTemplateColumns: '1fr 1fr', gap: '0.8rem', background: 'rgba(15,23,42,0.6)', padding: '1rem', borderRadius: '8px', marginBottom: '1.25rem' }}>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formDailyCheckin}
                    onChange={e => setFormDailyCheckin(e.target.checked)}
                  />
                  <span>{t('website.daily_checkin')}</span>
                </label>

                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formCheatAccount}
                    onChange={e => setFormCheatAccount(e.target.checked)}
                  />
                  <span>{t('website.cheat_account')}</span>
                </label>

                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formRequiresKyc}
                    onChange={e => setFormRequiresKyc(e.target.checked)}
                  />
                  <span>{t('website.requires_kyc')}</span>
                </label>

                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formRequiresProxy}
                    onChange={e => setFormRequiresProxy(e.target.checked)}
                  />
                  <span>{t('website.requires_proxy')}</span>
                </label>
              </div>

              {/* Custom Criteria Section */}
              <div className="form-group" style={{ borderTop: '1px solid var(--border)', paddingTop: '1rem' }}>
                <label style={{ display: 'flex', alignItems: 'center', gap: '0.4rem' }}>
                  <Sliders size={14} /> {t('website.custom_criteria')} (Người dùng tự thêm các tiêu chí khác)
                </label>
                
                <div style={{ display: 'flex', gap: '0.5rem', marginBottom: '0.75rem' }}>
                  <input
                    type="text"
                    className="form-control"
                    placeholder="Tên tiêu chí (vd: Cần 2FA, Giới hạn IP)"
                    value={newCritLabel}
                    onChange={e => setNewCritLabel(e.target.value)}
                    style={{ flex: 1 }}
                  />
                  <input
                    type="text"
                    className="form-control"
                    placeholder="Giá trị (vd: Bắt buộc, Tối đa 3)"
                    value={newCritValue}
                    onChange={e => setNewCritValue(e.target.value)}
                    style={{ flex: 1 }}
                  />
                  <button type="button" className="btn btn-secondary" onClick={handleAddCriterion}>
                    Thêm
                  </button>
                </div>

                <div style={{ display: 'flex', flexDirection: 'column', gap: '0.4rem' }}>
                  {formCustomCriteria.map((crit, idx) => (
                    <div key={idx} style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', background: 'rgba(255,255,255,0.05)', padding: '0.4rem 0.8rem', borderRadius: '6px' }}>
                      <span><strong>{crit.label}:</strong> {crit.value}</span>
                      <button type="button" onClick={() => handleRemoveCriterion(idx)} style={{ background: 'none', border: 'none', color: '#ef4444', cursor: 'pointer' }}>
                        <Trash2 size={14} />
                      </button>
                    </div>
                  ))}
                </div>
              </div>

              <div className="form-group">
                <label>{t('common.notes')}</label>
                <textarea
                  className="form-control"
                  rows={2}
                  value={formNotes}
                  onChange={e => setFormNotes(e.target.value)}
                  placeholder="Ghi chú thêm về quy định, tips hoặc cheat..."
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

      {selectedForEmails && (
        <WebsiteEmailsModal
          website={selectedForEmails}
          onClose={() => setSelectedForEmails(null)}
        />
      )}
    </div>
  );
};
