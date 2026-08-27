import re

with open("frontend/src/features/clipboard.ts", "r") as f:
    content = f.read()

# Add import
import_stmt = "import { listen } from '@tauri-apps/api/event';\n"
if "import { listen }" not in content:
    content = content.replace("import { invoke } from '@tauri-apps/api/core';", "import { invoke } from '@tauri-apps/api/core';\n" + import_stmt)

# Update pasteTo function
old_block = """    // Hiện thông báo đang kiểm tra xung đột đệ quy
    const checkingModal = new OperationModal('Đang kiểm tra...', '<p>Đang quét sâu để tìm các tệp tin trùng lặp...</p>');
    checkingModal.open();
    checkingModal.getElement().querySelector('.confirm')?.remove();
    checkingModal.getElement().querySelector('.cancel')?.remove();

    try {
      // 1. Quét sâu để lấy danh sách các tệp tin xung đột thực sự
      const conflictPaths: string[] = await invoke('fs_check_conflicts', { srcs, destPath });
      checkingModal.close();"""

new_block = """    // Hiện thông báo đang kiểm tra xung đột đệ quy
    const checkingModal = new OperationModal('Đang kiểm tra...', '<p>Đang quét sâu để tìm các tệp tin trùng lặp...</p>');
    checkingModal.open();
    checkingModal.getElement().querySelector('.confirm')?.remove();
    checkingModal.getElement().querySelector('.cancel')?.remove();

    let unlisten: any;
    try {
      unlisten = await listen('conflict_check_progress', (event: any) => {
          if (event.payload && event.payload.stats) {
              const stats = event.payload.stats;
              const content = checkingModal.getElement().querySelector('.modal-content p');
              if (content) {
                  content.innerHTML = `Đang quét sâu để tìm các tệp tin trùng lặp...<br>Đã kiểm tra: ${stats.checks} tệp<br>Tốc độ: ${Math.round(stats.speed / 1024)} KB/s<br>Thời gian: ${Math.round(stats.elapsedTime)}s`;
              }
          }
      });
      // 1. Quét sâu để lấy danh sách các tệp tin xung đột thực sự
      const conflictPaths: string[] = await invoke('fs_check_conflicts', { srcs, destPath });
      if (unlisten) unlisten();
      checkingModal.close();"""

content = content.replace(old_block, new_block)

with open("frontend/src/features/clipboard.ts", "w") as f:
    f.write(content)
print("Success")
