/*
[INTEGRITY NOTES]
- Mục đích: Trang quản lý danh sách Gói dịch vụ.
- Trách nhiệm: Hiển thị bảng dữ liệu, gọi PackageModal để thêm/sửa, và xóa Package.
- Tương tác: Dùng `usePackageStore` để lấy và thao tác state.
*/

import { useEffect, useState } from 'react';
import { usePackageStore } from '../store/usePackageStore';
import { PackageModal } from '../components/PackageModal';
import { ConfirmModal } from '../components/ConfirmModal';
import { Plus, Edit, Trash2 } from 'lucide-react';
import type { Package } from '../../../bridge/types';
import { useTranslation, formatCurrency } from '../utils/i18n';

export function PackageManagementPage() {
    const { t } = useTranslation();
    const { packages, isLoading, fetchPackages, addNewPackage, editPackage, removePackage } = usePackageStore();
    
    // Quản lý Modal
    const [isModalOpen, setIsModalOpen] = useState(false);
    const [selectedPackage, setSelectedPackage] = useState<Package | null>(null);

    // Quản lý Confirm Delete Modal
    const [isConfirmOpen, setIsConfirmOpen] = useState(false);
    const [pkgToDelete, setPkgToDelete] = useState<string | null>(null);

    // Tải dữ liệu lần đầu
    useEffect(() => {
        fetchPackages();
    }, [fetchPackages]);

    // Mở modal thêm mới
    const handleAddClick = () => {
        setSelectedPackage(null);
        setIsModalOpen(true);
    };

    // Mở modal sửa
    const handleEditClick = (pkg: Package) => {
        setSelectedPackage(pkg);
        setIsModalOpen(true);
    };

    // Xóa gói
    const requestDelete = (id: string) => {
        setPkgToDelete(id);
        setIsConfirmOpen(true);
    };

    const confirmDelete = async () => {
        if (pkgToDelete) {
            await removePackage(pkgToDelete);
            setIsConfirmOpen(false);
            setPkgToDelete(null);
        }
    };

    // Xử lý lưu từ Modal
    const handleSave = async (name: string, duration_days: number, price: number, description?: string) => {
        if (selectedPackage) {
            await editPackage(selectedPackage.id, name, duration_days, price, description);
        } else {
            await addNewPackage(name, duration_days, price, description);
        }
    };

    return (
        <>
        <div className="animate-fade-in">
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '2rem' }}>
                <h1>{t('packages.title')}</h1>
                <button className="btn btn-primary" onClick={handleAddClick}>
                    <Plus size={18} /> {t('packages.add_pkg')}
                </button>
            </div>

            <div className="glass-panel">
                {isLoading && packages.length === 0 ? (
                    <p>{t('common.loading')}</p>
                ) : (
                    <div className="table-container">
                        <table>
                            <thead>
                                <tr>
                                    <th>ID</th>
                                    <th>{t('packages.pkg_name')}</th>
                                    <th>{t('packages.duration')}</th>
                                    <th>{t('packages.price')}</th>
                                    <th>{t('packages.desc')}</th>
                                    <th>{t('packages.actions')}</th>
                                </tr>
                            </thead>
                            <tbody>
                                {packages.length === 0 ? (
                                    <tr><td colSpan={6} style={{ textAlign: 'center' }}>{t('packages.no_pkgs')}</td></tr>
                                ) : (
                                    packages.map(pkg => (
                                        <tr key={pkg.id}>
                                            <td style={{ color: 'var(--text-secondary)' }}>{pkg.id}</td>
                                            <td style={{ fontWeight: 600 }}>{pkg.name}</td>
                                            <td>{pkg.duration_days}</td>
                                            <td style={{ color: 'var(--success-color, #34d399)' }}>{pkg.price ? formatCurrency(pkg.price) : '0 đ'}</td>
                                            <td style={{ color: 'var(--text-secondary)' }}>{pkg.description || '-'}</td>
                                            <td>
                                                <div style={{ display: 'flex', gap: '0.5rem' }}>
                                                    <button className="btn btn-primary" style={{ padding: '0.4rem 0.6rem' }} onClick={() => handleEditClick(pkg)}>
                                                        <Edit size={16} />
                                                    </button>
                                                    <button className="btn btn-danger" style={{ padding: '0.4rem 0.6rem' }} onClick={() => requestDelete(pkg.id)}>
                                                        <Trash2 size={16} />
                                                    </button>
                                                </div>
                                            </td>
                                        </tr>
                                    ))
                                )}
                            </tbody>
                        </table>
                    </div>
                )}
            </div>
        </div>

            {/* Gọi Component Modal */}
            <PackageModal 
                isOpen={isModalOpen} 
                packageData={selectedPackage} 
                onClose={() => setIsModalOpen(false)} 
                onSave={handleSave} 
            />

            {/* Modal Xác nhận Xóa */}
            <ConfirmModal 
                isOpen={isConfirmOpen}
                title="Xác nhận Xóa Gói dịch vụ"
                message="Bạn có chắc chắn muốn xóa gói này? Dữ liệu sẽ bị xóa vĩnh viễn và không thể khôi phục."
                onConfirm={confirmDelete}
                onCancel={() => {
                    setIsConfirmOpen(false);
                    setPkgToDelete(null);
                }}
                isDanger={true}
            />
        </>
    );
}
