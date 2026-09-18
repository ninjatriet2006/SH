# Debate Brief (đọc 1 lần, dùng mọi vòng — gọn để tiết kiệm context)

Dự án: `GUI/vpn_ai_proxy_gui` — Tauri v2 (Rust backend `src/`: config, proxy/{mod,routing,rotation,upstream,manager,key_manager}, vpn/{types,network,process}, monitor/{types,disk}, fingerprint/{types,patterns,analyzer,sanitizer}, commands, workers, state, lib) + React/TS frontend (`src/`: App, hooks/useConfig-useTraffic-useTunnels, components/*).

## Verdicts đã chốt (đừng đào lại, chỉ tìm lỗi MỚI)
1. Giữ 503 khi tunnel sập (không hàng đợi boot); 503 có Retry-After + X-Tunnel-State + JSON body.
2. Tunnel CLI `Unknown` không được route (chỉ `Online`, hoặc Unknown-không-cần-process).
3. Boot reset sạch health về Unknown/Offline (cả tunnels lẫn routes).
4. Key file chỉ đốt sau N lỗi 401/403 liên tiếp (`max_key_failures`, default 3); token tay không đụng file.
5. `mask_local_paths_in_body` che path thật trên outbound body, log riêng `raw_forwarded_body`.
6. Mọi writer Offline+error đều xóa IP/latency cũ (chống ghost IP xanh).
7. Disk I/O qua worker riêng; success-reset dùng upgradable-read; reqwest có connect_timeout 30s.
8. Frontend: badge route 4 trạng thái (Unknown vàng), error ưu tiên hơn IP, Test disable khi tunnel tắt.

## Luật debate (bắt buộc)
- NOCODE tuyệt đối: chỉ đọc + phân tích, không sửa/tạo file, không chạy lệnh ghi.
- ĐỌC TOÀN DIỆN bằng GitNexus CLI (binary global, chạy từ /home/bimatkeo/Documents/SH):
  `gitnexus context -r SH <symbol>` cho mọi symbol định bàn (callers + callees),
  `gitnexus impact -r SH <symbol>` trước khi đề xuất sửa nó. CẤM kết luận từ đọc rời rạc.
- CHỐNG ẢO GIÁC (bắt buộc mở đầu mọi vòng): trích NGUYÊN VĂN câu hỏi/đề mục đang phản biện
  từ file brief (chứng minh đã đọc file thật, không bịa Proposal mới). Vòng nào đặt tên
  design không có trong brief (vd "CircuitBreaker", "Leaky Bucket" trong khi brief ghi
  fixed-interval) thì vòng đó HỦY, không tính.
- PROOF-OF-READING bằng lệnh (bắt buộc khi review code): mở đầu response bằng
  (1) output `wc -l` của từng file đã đọc, (2) chữ ký nguyên văn (verbatim signature)
  của hàm chính đang bàn, (3) trả lời 2-3 câu hỏi gài sẵn trong prompt (đáp án nằm
  giữa file, không phải đầu file). Thiếu 1 trong 3 = vòng HỦY.
- Output: tối đa 10 mục `[ĐỒNG Ý/PHẢN ĐỐI/HỎI] — 1-2 dòng`, tiếng Việt, ngắn, ánh xạ đúng
  số mục trong file (mục 1-7, Q1-Q3 của debate_pacing.md).
- Bỏ qua 8 verdicts trên trừ khi tìm ra bằng chứng chúng sai.
