import React, { useState } from 'react';
import { useAppStore } from '../store';
import { LoginMethod } from '../../../bridge/types';
import { Plus, Trash2, Edit } from 'lucide-react';

export const LoginMethodsPage: React.FC = () => {
  const { data, saveLoginMethod, deleteLoginMethod } = useAppStore();
  const [modalOpen, setModalOpen] = useState(false);
  const [editing, setEditing] = useState<LoginMethod | null>(null);
  const [formName, setFormName] = useState('');
  const [formDesc, setFormDesc] = useState('');

  if (!data) return null;

  const openAdd = () => {
    setEditing(null);
    setFormName('');
    setFormDesc('');
    setModalOpen(true);
  };

  const openEdit = (m: LoginMethod) => {
    setEditing(m);
    setFormName(m.name);
    setFormDesc(m.description || '');
    setModalOpen(true);
  };

  const handleSave = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!formName.trim()) return;
    await saveLoginMethod({
      id: editing ? editing.id : '',
      name: formName.trim(),
      description: formDesc.trim() || undefined,
      created_at: editing ? editing.created_at : '',
    });
    setModalOpen(false);
  };

  const handleDelete = async (id: string) => {
    if (window.confirm('Xóa phương thức đăng nhập này? Gán trên các website sẽ được gỡ.')) {
      await deleteLoginMethod(id);
    }
  };

  const usageCount = (id: string) => data.websites.filter(w => (w.login_method_ids || []).includes(id)).length;

  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: '1.5rem' }}>
      <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
        <div>
          <h1 style={{ fontSize: '1.8rem', fontWeight: 700 }}>Phương thức đăng nhập</h1>
          <p style={{ color: 'var(--text-secondary)' }}>Quản lý phương thức đăng nhập gán cho website qua dropdown</p>
        </div>
        <button className="btn btn-primary" onClick={openAdd}>
          <Plus size={18} /> Thêm
        </button>
      </div>

      <div className="glass-card" style={{ padding: 0, overflow: 'hidden' }}>
        <div className="table-container">
          <table>
            <thead>
              <tr>
                <th>Tên phương thức</th>
                <th>Mô tả</th>
                <th>Đang gán</th>
                <th style={{ textAlign: 'right' }}>Thao tác</th>
              </tr>
            </thead>
            <tbody>
              {(data.login_methods || []).map(m => (
                <tr key={m.id}>
                  <td style={{ fontWeight: 600 }}>{m.name}</td>
                  <td style={{ color: 'var(--text-secondary)', fontSize: '0.85rem' }}>{m.description || '—'}</td>
                  <td><span className="badge badge-gray">{usageCount(m.id)} web</span></td>
                  <td style={{ textAlign: 'right' }}>
                    <div style={{ display: 'flex', gap: '0.5rem', justifyContent: 'flex-end' }}>
                      <button className="btn btn-secondary" style={{ padding: '0.4rem 0.6rem' }} onClick={() => openEdit(m)}>
                        <Edit size={14} />
                      </button>
                      <button className="btn btn-danger" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleDelete(m.id)}>
                        <Trash2 size={14} />
                      </button>
                    </div>
                  </td>
                </tr>
              ))}
              {(data.login_methods || []).length === 0 && (
                <tr><td colSpan={4} style={{ textAlign: 'center', padding: '2rem', color: 'var(--text-secondary)' }}>Chưa có phương thức nào</td></tr>
              )}
            </tbody>
          </table>
        </div>
      </div>

      {modalOpen && (
        <div className="modal-overlay" onClick={() => setModalOpen(false)}>
          <div className="modal-content" onClick={e => e.stopPropagation()}>
            <h2 style={{ marginBottom: '1.5rem' }}>{editing ? 'Chỉnh sửa phương thức' : 'Thêm phương thức'}</h2>
            <form onSubmit={handleSave}>
              <div className="form-group">
                <label>Tên phương thức *</label>
                <input type="text" className="form-control" required value={formName} onChange={e => setFormName(e.target.value)} placeholder="Ví dụ: Email + Password, Wallet..." />
              </div>
              <div className="form-group">
                <label>Mô tả</label>
                <input type="text" className="form-control" value={formDesc} onChange={e => setFormDesc(e.target.value)} placeholder="Mô tả ngắn (không bắt buộc)" />
              </div>
              <div style={{ display: 'flex', gap: '1rem', justifyContent: 'flex-end', marginTop: '1.5rem' }}>
                <button type="button" className="btn btn-secondary" onClick={() => setModalOpen(false)}>Hủy</button>
                <button type="submit" className="btn btn-primary">Lưu</button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
