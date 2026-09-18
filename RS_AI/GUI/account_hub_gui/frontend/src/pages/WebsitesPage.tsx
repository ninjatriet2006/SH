import React, { useState } from 'react';
import { useAppStore } from '../store';
import { useTranslation } from '../i18n';
import { Website } from '../../../bridge/types';
import { Plus, Trash2, Edit, Globe, ExternalLink, Users } from 'lucide-react';
import { WebsiteEmailsModal } from '../components/WebsiteEmailsModal';
import { MultiSelectDropdown } from '../components/MultiSelectDropdown';

export const WebsitesPage: React.FC = () => {
  const { data, saveWebsite, deleteWebsite } = useAppStore();
  const { t } = useTranslation();
  const [modalOpen, setModalOpen] = useState(false);
  const [editingWeb, setEditingWeb] = useState<Website | null>(null);
  const [selectedForEmails, setSelectedForEmails] = useState<Website | null>(null);

  const [formName, setFormName] = useState('');
  const [formUrl, setFormUrl] = useState('');
  const [formTags, setFormTags] = useState('');
  const [formDailyCheckin, setFormDailyCheckin] = useState(false);
  const [formCriterionIds, setFormCriterionIds] = useState<string[]>([]);
  const [formLoginMethodIds, setFormLoginMethodIds] = useState<string[]>([]);
  const [formNotes, setFormNotes] = useState('');

  if (!data) return null;

  const criteria = data.criteria || [];
  const loginMethods = data.login_methods || [];
  const criterionName = (id: string) => criteria.find(c => c.id === id)?.name || id;
  const loginMethodName = (id: string) => loginMethods.find(m => m.id === id)?.name || id;

  const handleOpenAdd = () => {
    setEditingWeb(null);
    setFormName('');
    setFormUrl('');
    setFormTags('');
    setFormDailyCheckin(false);
    setFormCriterionIds([]);
    setFormLoginMethodIds([]);
    setFormNotes('');
    setModalOpen(true);
  };

  const handleOpenEdit = (w: Website) => {
    setEditingWeb(w);
    setFormName(w.name);
    setFormUrl(w.url);
    setFormTags((w.tags || []).join(', '));
    setFormDailyCheckin(w.has_daily_checkin);
    setFormCriterionIds([...(w.criterion_ids || [])]);
    setFormLoginMethodIds([...(w.login_method_ids || [])]);
    setFormNotes(w.notes);
    setModalOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) return;

    const webObj: Website = {
      id: editingWeb ? editingWeb.id : '',
      name: formName.trim(),
      url: formUrl.trim(),
      tags: formTags.split(',').map(s => s.trim()).filter(Boolean),
      has_daily_checkin: formDailyCheckin,
      criterion_ids: formCriterionIds,
      login_method_ids: formLoginMethodIds,
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
          <p style={{ color: 'var(--text-secondary)' }}>Danh sách website/nền tảng, gán tiêu chí và phương thức đăng nhập qua dropdown</p>
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
                <th>Tags</th>
                <th>Tiêu chí / Đăng nhập</th>
                <th>Gmail Đã Gán (Live / Die)</th>
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
                      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                        {(web.tags || []).map(tag => (
                          <span key={tag} className="badge badge-gray">{tag}</span>
                        ))}
                        {(web.tags || []).length === 0 && <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>—</span>}
                      </div>
                    </td>
                    <td>
                      <div style={{ display: 'flex', gap: '4px', flexWrap: 'wrap' }}>
                        {(web.criterion_ids || []).map(id => (
                          <span key={id} className="badge badge-success" style={{ fontSize: '0.7rem' }}>{criterionName(id)}</span>
                        ))}
                        {(web.login_method_ids || []).map(id => (
                          <span key={id} className="badge badge-warning" style={{ fontSize: '0.7rem' }}>{loginMethodName(id)}</span>
                        ))}
                        {(web.criterion_ids || []).length === 0 && (web.login_method_ids || []).length === 0 && (
                          <span style={{ color: 'var(--text-secondary)', fontSize: '0.8rem' }}>—</span>
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
                  <label>Tags (phân cách bằng dấu phẩy)</label>
                  <input
                    type="text"
                    className="form-control"
                    value={formTags}
                    onChange={e => setFormTags(e.target.value)}
                    placeholder="Crypto, DePIN, Social..."
                  />
                </div>
              </div>

              <div style={{ background: 'rgba(15,23,42,0.6)', padding: '1rem', borderRadius: '8px', marginBottom: '1.25rem' }}>
                <label className="checkbox-label">
                  <input
                    type="checkbox"
                    checked={formDailyCheckin}
                    onChange={e => setFormDailyCheckin(e.target.checked)}
                  />
                  <span>{t('website.daily_checkin')}</span>
                </label>
              </div>

              <div className="form-group">
                <label>Tiêu chí</label>
                <MultiSelectDropdown
                  placeholder="Chọn tiêu chí cho web này..."
                  options={criteria.map(c => ({ id: c.id, name: c.name }))}
                  selectedIds={formCriterionIds}
                  onChange={setFormCriterionIds}
                  emptyHint="Chưa có tiêu chí — tạo ở trang Tiêu chí."
                />
              </div>

              <div className="form-group">
                <label>Phương thức đăng nhập</label>
                <MultiSelectDropdown
                  placeholder="Chọn phương thức web này hỗ trợ..."
                  options={loginMethods.map(m => ({ id: m.id, name: m.name }))}
                  selectedIds={formLoginMethodIds}
                  onChange={setFormLoginMethodIds}
                  emptyHint="Chưa có phương thức — tạo ở trang Đăng nhập."
                />
              </div>

              <div className="form-group">
                <label>{t('common.notes')}</label>
                <textarea
                  className="form-control"
                  rows={2}
                  value={formNotes}
                  onChange={e => setFormNotes(e.target.value)}
                  placeholder="Ghi chú thêm về quy định, tips..."
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
