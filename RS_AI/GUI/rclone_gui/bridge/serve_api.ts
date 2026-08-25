/*
[INTEGRITY NOTES]
Mục đích: Khai báo API giao tiếp Tauri/Backend cho việc Serve (WebDAV, FTP, HTTP).
Trách nhiệm: Khởi chạy serve daemon.
Các module tương tác: frontend/src/main.ts, backend/src/serve.rs
*/

export async function serveRemote(remote: string, protocol: string, port: number): Promise<boolean> {
  // TODO: window.__TAURI__.invoke
  return true;
}
