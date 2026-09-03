/*
[INTEGRITY NOTES]
- Mục đích: Trình sửa nội dung văn bản tối giản cho một tệp đơn.
- Trách nhiệm: Đọc nội dung qua `fileOps.read`, cho phép chỉnh sửa, ghi lại qua
  `fileOps.write`. Hoạt động với cả ổ Local và remote (backend dùng rclone cat/rcat).
- Tương tác: Mở từ context menu ("Sửa nội dung (Text)"). Gọi `onSaved` sau khi
  ghi thành công để pane nạp lại (kích thước / thời gian sửa đã đổi).
*/
import { OperationModal } from './OperationModal';
import * as fileOps from '../services/fileOps';
import { escapeHtml } from '../features/format';

export class TextEditorModal {
  private modal: OperationModal;
  private fullPath: string;
  private onSaved: () => Promise<void>;
  private original = '';

  constructor(fileName: string, fullPath: string, onSaved: () => Promise<void>) {
    this.fullPath = fullPath;
    this.onSaved = onSaved;
    this.modal = new OperationModal(`Sửa: ${fileName}`, this.getLoadingHtml());
  }

  private getLoadingHtml(): string {
    return `<div style="padding: 20px; text-align: center; color: var(--colors-text-muted);">Đang tải nội dung…</div>`;
  }

  private getEditorHtml(content: string): string {
    return `
      <div style="display: flex; flex-direction: column; gap: 8px; width: 640px; max-width: 85vw;">
        <div style="font-size: 12px; color: var(--colors-text-muted); white-space: nowrap; overflow: hidden; text-overflow: ellipsis;">
          ${escapeHtml(this.fullPath)}
        </div>
        <textarea id="text-editor-area" spellcheck="false" style="height: 420px; max-height: 60vh; resize: vertical; padding: 10px; border-radius: 4px; border: 1px solid var(--colors-border-muted); background: var(--colors-surface-input); color: var(--colors-text-primary); font-family: var(--typography-font-mono, monospace); font-size: 13px; line-height: 1.5;">${escapeHtml(content)}</textarea>
        <div id="text-editor-status" style="font-size: 12px; color: var(--colors-text-muted); min-height: 16px;"></div>
      </div>
    `;
  }

  public async open(): Promise<void> {
    this.modal.open();

    const confirmBtn = this.modal.getElement().querySelector('.confirm') as HTMLButtonElement | null;
    if (confirmBtn) {
      confirmBtn.textContent = 'Lưu';
      confirmBtn.disabled = true; // Bật sau khi tải xong
    }

    try {
      this.original = await fileOps.read(this.fullPath);
    } catch (e) {
      // File nhị phân hoặc quá lớn — backend đã trả thông điệp cụ thể.
      this.modal.getBody().innerHTML =
        `<div style="padding: 20px; color: var(--colors-neon-coral, red); max-width: 520px;">${escapeHtml(String(e))}</div>`;
      if (confirmBtn) confirmBtn.style.display = 'none';
      return;
    }

    this.modal.getBody().innerHTML = this.getEditorHtml(this.original);

    const area = this.modal.getElement().querySelector('#text-editor-area') as HTMLTextAreaElement | null;
    const status = this.modal.getElement().querySelector('#text-editor-status') as HTMLDivElement | null;
    if (!area) return;

    area.focus();
    if (confirmBtn) confirmBtn.disabled = true;

    // Chỉ cho Lưu khi nội dung thực sự đổi.
    area.addEventListener('input', () => {
      const changed = area.value !== this.original;
      if (confirmBtn) confirmBtn.disabled = !changed;
      if (status) status.textContent = changed ? 'Đã thay đổi — chưa lưu' : '';
    });

    confirmBtn?.addEventListener('click', async () => {
      if (area.value === this.original) {
        this.modal.close();
        return;
      }
      confirmBtn.disabled = true;
      if (status) status.textContent = 'Đang lưu…';
      try {
        await fileOps.write(this.fullPath, area.value);
        this.modal.close();
        await this.onSaved();
      } catch (e) {
        if (status) {
          status.style.color = 'var(--colors-neon-coral, red)';
          status.textContent = `Lưu thất bại: ${String(e)}`;
        }
        confirmBtn.disabled = false;
      }
    });
  }
}
