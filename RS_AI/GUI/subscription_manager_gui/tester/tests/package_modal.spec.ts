import { test, expect } from '@playwright/test';

test.describe('PackageModal Edge Cases & Workflows', () => {
  test.beforeEach(async ({ page }) => {
    // Truy cập trang quản lý gói dịch vụ
    await page.goto('/packages');
    // Chờ cho ứng dụng tải
    await page.waitForSelector('text=Quản lý Gói dịch vụ');
  });

  test('Test_Flow_Hoan_Thien_Package_Modal: Điền thông tin và lưu thành công', async ({ page }) => {
    // Click nút thêm gói mới
    await page.click('text=Thêm Gói Mới');

    // Chờ modal xuất hiện
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeVisible();

    // Điền thông tin hợp lệ
    await page.fill('input[placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"]', 'Gói VIP 1 Năm');
    
    // Ghi đè số ngày (mặc định là 30)
    await page.fill('input[type="number"]', '365');
    
    // Ghi mô tả
    await page.fill('textarea[placeholder="Nhập thông tin chi tiết về gói..."]', 'Gói dành cho thành viên VIP');

    // Nhấn lưu cấu hình
    await page.click('button[type="submit"]:has-text("Lưu cấu hình")');

    // Modal phải bị đóng (không còn visible) - Do đã fix lỗi backend mapping
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeHidden();
  });

  test('Test_Flow_Huy_Bo_Reset_State_Package: Hủy tạo và kiểm tra state reset', async ({ page }) => {
    // Click nút thêm gói mới
    await page.click('text=Thêm Gói Mới');
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeVisible();

    // Nhập dở thông tin tên gói
    await page.fill('input[placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"]', 'Dữ liệu chưa hoàn thành');

    // Nhấn Hủy
    await page.click('button:has-text("Hủy")');
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeHidden();

    // Mở lại Modal
    await page.click('text=Thêm Gói Mới');
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeVisible();

    // Kiểm tra ô input tên gói đã bị reset trắng
    const inputValue = await page.inputValue('input[placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"]');
    expect(inputValue).toBe('');
    
    // Kiểm tra ô input số ngày đã reset về 30
    const durationValue = await page.inputValue('input[type="number"]');
    expect(durationValue).toBe('30');
  });

  test('Test_Flow_Bo_Trong_Truong_Bat_Buoc_Package: Validation của form', async ({ page }) => {
    await page.click('text=Thêm Gói Mới');
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeVisible();

    // Ô tên gói đang trống mặc định, cố tình để trống
    await page.fill('input[placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"]', '');

    // Nhấn Lưu
    await page.click('button[type="submit"]:has-text("Lưu cấu hình")');

    // Do trường tên gói có required, form sẽ chặn submit và modal không đóng
    await expect(page.getByText('Tạo Gói Dịch Vụ Mới')).toBeVisible();
    
    // Kiểm tra validity của ô input tên gói
    const isInvalid = await page.$eval('input[placeholder="Ví dụ: Gói Cơ Bản 1 Tháng"]', (el: HTMLInputElement) => !el.validity.valid);
    expect(isInvalid).toBe(true);
  });
});
