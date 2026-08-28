import { tempDir, join } from '@tauri-apps/api/path';
async function test() {
  const dir = await tempDir();
  console.log(await join(dir, "rclone_gui_temp"));
}
