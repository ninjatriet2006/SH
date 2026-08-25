//! [INTEGRITY NOTES]
//! Mục đích: Nhóm các Tauri commands liên quan đến hàng đợi truyền tải (Transfer).
//! Trách nhiệm: Thêm, Hủy, Bắt đầu transfer, giao tiếp (event emitter) tiến trình download/upload/copy/move.
//! Tương tác: Giao tiếp với `TransferManager` trong `AppState`.

use crate::state::{AppState, TransferProgressPayload, TransferFinishedPayload};
use filen_gui::transfer::{
    ProgressUpdate, TransferError, TransferItem, TransferKind, TransferStatus,
    copy_local, delete_local_path, move_local, run_cli_transfer_terminal,
};
use tauri::Emitter;

/// Chuyển đổi từ chuỗi (string) cấu hình sang kiểu enum `TransferKind` an toàn.
pub fn parse_transfer_kind(kind: &str) -> Result<TransferKind, String> {
    match kind {
        "upload" => Ok(TransferKind::Upload),
        "download" => Ok(TransferKind::Download),
        "copy" => Ok(TransferKind::Copy),
        "move" => Ok(TransferKind::Move),
        _ => Err(format!("Không biết loại truyền tải: {kind}")),
    }
}

/// Đưa một tác vụ vào hàng đợi. Trả về ID của tác vụ.
/// Tác vụ chưa được chạy cho đến khi `transfer_start` được gọi.
#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub fn transfer_enqueue(
    state: tauri::State<'_, AppState>,
    kind: String,
    name: String,
    src: String,
    dst: String,
    src_local: bool,
    dst_local: bool,
    cleanup_src: bool,
    src_pane: usize,
    dst_pane: usize,
) -> Result<usize, String> {
    let kind = parse_transfer_kind(&kind)?;
    // Khóa trạng thái để sửa đổi danh sách tác vụ an toàn giữa các luồng
    let mut mgr = state.transfer.lock().map_err(|e| e.to_string())?;
    Ok(mgr.enqueue(
        kind,
        name,
        src,
        dst,
        src_local,
        dst_local,
        cleanup_src,
        src_pane,
        dst_pane,
    ))
}

/// Bắt đầu xử lý các tác vụ đang đợi cho đến khi đạt giới hạn chạy song song (`max_concurrent`).
/// Mỗi tác vụ sẽ tạo một luồng ảo (async task) riêng để không chặn luồng chính.
#[tauri::command]
pub async fn transfer_start(
    app: tauri::AppHandle,
    state: tauri::State<'_, AppState>,
    account: Option<String>,
) -> Result<(), String> {
    let (batch, timeout_secs) = {
        let mut mgr = state.transfer.lock().map_err(|e| e.to_string())?;
        let timeout_secs = mgr.timeout_secs;
        let mut batch = Vec::new();
        
        // Kích hoạt các tác vụ tiếp theo nếu chưa quá số lượng tối đa cho phép
        while mgr.running_count() < mgr.max_concurrent {
            let Some(idx) = mgr.next_queued_idx() else {
                break; // Hết tác vụ đang đợi
            };
            let item = mgr.items[idx].clone();
            mgr.items[idx].status = TransferStatus::Running; // Đánh dấu là đang chạy
            batch.push(item);
        }
        (batch, timeout_secs)
    };
    
    // Spawn từng tác vụ độc lập vào background
    for item in batch {
        let app = app.clone();
        let account = account.clone();
        tauri::async_runtime::spawn(async move {
            run_transfer_worker(app, item, account, timeout_secs).await;
        });
    }
    Ok(())
}

/// Hủy một tác vụ thông qua ID.
#[tauri::command]
pub fn transfer_cancel(state: tauri::State<'_, AppState>, id: usize) -> Result<(), String> {
    let mgr = state.transfer.lock().map_err(|e| e.to_string())?;
    mgr.cancel(id);
    Ok(())
}

/// Hủy toàn bộ tác vụ trong hàng đợi.
#[tauri::command]
pub fn transfer_cancel_all(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mgr = state.transfer.lock().map_err(|e| e.to_string())?;
    mgr.cancel_all();
    Ok(())
}

/// Dọn dẹp danh sách các tác vụ đã hoàn thành hoặc thất bại (để giải phóng UI).
#[tauri::command]
pub fn transfer_remove_finished(state: tauri::State<'_, AppState>) -> Result<(), String> {
    let mut mgr = state.transfer.lock().map_err(|e| e.to_string())?;
    mgr.remove_finished();
    Ok(())
}

/// Xử lý cốt lõi luồng truyền tải thực sự, sau đó báo cáo kết quả (event emitter) về UI.
pub async fn run_transfer_worker(
    app: tauri::AppHandle,
    item: TransferItem,
    account: Option<String>,
    timeout_secs: u64,
) {
    let id = item.id;
    let kind = item.kind;
    let src = item.src.clone();
    let dst = item.dst.clone();
    let src_local = item.src_local;
    let dst_local = item.dst_local;
    let cancelled = item.cancelled.clone();
    let cleanup_src = item.cleanup_src;

    // Callback báo cáo tiến trình (bắn sự kiện qua IPC về frontend)
    let on_update = {
        let app = app.clone();
        move |upd: ProgressUpdate| {
            let payload = TransferProgressPayload {
                id,
                progress: upd.progress,
                bytes_done: upd.bytes_done,
                total_bytes: upd.total_bytes,
            };
            let _ = app.emit("transfer:progress", payload);
        }
    };

    // Xác định logic cần chạy tùy theo loại truyền tải
    let result = match kind {
        // Upload/Download đi qua CLI
        TransferKind::Upload | TransferKind::Download => {
            run_cli_transfer_terminal(kind, &src, &dst, timeout_secs, cancelled, on_update).await
        }
        TransferKind::Copy | TransferKind::Move => {
            if !src_local && !dst_local {
                // Di chuyển/Sao chép từ Cloud -> Cloud (trên server)
                let res = match kind {
                    TransferKind::Copy => filen_gui::cloud_fs::cp_terminal(&account, &src, &dst).await,
                    TransferKind::Move => filen_gui::cloud_fs::mv_terminal(&account, &src, &dst).await,
                    _ => unreachable!("Chỉ áp dụng cho Copy/Move"),
                };
                res.map_err(TransferError::Failed)
            } else if src_local && dst_local {
                // Di chuyển/Sao chép nội bộ máy (Local -> Local)
                let res = match kind {
                    TransferKind::Copy => copy_local(&src, &dst),
                    TransferKind::Move => move_local(&src, &dst),
                    _ => unreachable!("Chỉ áp dụng cho Copy/Move"),
                };
                res.map_err(TransferError::Failed)
            } else {
                Err(TransferError::Spawn(
                    "Thao tác sao chép hoặc di chuyển không hỗ trợ giữa hai hệ thống khác loại (vd: giữa Cloud và Local)".to_string(),
                ))
            }
        }
    };

    // Nếu thao tác di chuyển thành công và yêu cầu dọn dẹp nguồn, ta tiến hành xóa file/thư mục gốc
    if result.is_ok() && cleanup_src {
        let cleanup = if src_local {
            delete_local_path(&src)
        } else {
            filen_gui::cloud_fs::rm_terminal(&account, &src, true).await
        };
        if let Err(e) = cleanup {
            let _ = app.emit(
                "transfer:finished",
                TransferFinishedPayload {
                    id,
                    ok: false,
                    error: Some(format!("Đã truyền tải xong nhưng không thể xóa file gốc: {e}")),
                },
            );
            return; // Lỗi khi xoá nguồn coi như toàn bộ quy trình move chưa hoàn hảo
        }
    }

    // Báo cáo thành công hay thất bại cuối cùng về UI
    let (ok, error) = match &result {
        Ok(_) => (true, None),
        Err(e) => (false, Some(e.to_string())),
    };
    let _ = app.emit("transfer:finished", TransferFinishedPayload { id, ok, error });
}
