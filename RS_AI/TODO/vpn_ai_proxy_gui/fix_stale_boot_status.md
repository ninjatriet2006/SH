# Đặc tả lỗi State ảo lúc khởi động (Stale Boot Status)

## Hiện tượng (Symptom)
Theo như ảnh chụp màn hình, mặc dù VPN chưa thực sự bật hoặc quá trình khởi động ngầm chưa hoàn tất/thất bại, nhưng giao diện lại hiển thị badge **`ONLINE`** màu xanh lá. Dữ liệu này không phản ánh thực tế mạng lúc đó.

## Nguyên nhân gốc rễ (Root Cause)
1. **Lưu trạng thái Runtime vào ổ cứng:** Khi ứng dụng tắt, nó lưu toàn bộ struct `GatewayConfig` xuống file `config.json`. Struct này chứa trường `status` (trạng thái mạng: Online/Offline/Unknown) và các trường runtime khác (`last_exit_ip`, `last_latency_ms`). Nếu app bị tắt lúc VPN đang Online, file JSON sẽ ghi cứng chữ `"Online"`.
2. **Khôi phục trạng thái mù quáng:** Khi mở app lại, hàm `GatewayConfig::load_or_default()` (trong `backend/src/config.rs`) bốc nguyên xi dữ liệu từ `config.json` nhét vào bộ nhớ. 
3. **Hiển thị quá sớm:** Frontend lập tức fetch config này lúc giao diện vừa vẽ xong. Lúc này, tiến trình con (VPN CLI) **thực tế đã chết từ phiên làm việc trước và chưa kịp được start lại** (vì auto-start worker bị delay 500ms), nhưng UI đọc thấy chữ `Online` từ config cũ nên lập tức in ra cái badge màu xanh giả tạo.

## Cách giải quyết (TODO Tasks)
Cần thiết lập quy tắc (invariant): **Mọi trạng thái sức khỏe (Status, IP, Latency) KHÔNG được phép tin tưởng từ ổ cứng khi vừa boot lên, mà phải dựa vào thực tế chạy.**

- [x] Mở file `backend/src/config.rs`.
- [x] Trong hàm `load_or_default()`, ngay sau khi deserialize thành công `GatewayConfig` (khoảng dòng 113), bổ sung một vòng lặp quét qua tất cả `cfg.tunnels`.
- [x] Ép (Reset) toàn bộ các trường mang tính thời điểm về giá trị mặc định lúc khởi động:
  ```rust
  for t in &mut cfg.tunnels {
      // Nếu enabled thì để Unknown (chờ worker start rồi tính), nếu disabled thì Offline
      t.status = if t.enabled { crate::vpn::TunnelStatus::Unknown } else { crate::vpn::TunnelStatus::Offline };
      t.last_exit_ip = None;
      t.last_latency_ms = None;
      t.last_error = None;
  }
  ```
- [x] Chỉ cần thay đổi này, lúc boot lên UI sẽ hiện `Unknown` (màu xám) thay vì `Online` láo. Vài giây sau khi worker chạy xong, nó sẽ tự update thành `Online` thật sự nếu thành công, hoặc `Offline` nếu lỗi.

## Review độc lập (không tin mù spec)
- Chuỗi root-cause đã đối chiếu code (`config.rs:143` persist full struct, `config.rs:112-118` load mù, `workers.rs:12` delay 500ms + start/test mất thêm vài giây) — **triệu chứng có thật**, core fix đúng.
- Đã hiệu chỉnh 2 thiếu sót của spec khi implement:
  1. Reset luôn `last_checked_at` (spec quên → timestamp "Checked 2 ngày trước" treo cạnh badge Unknown là vênh A-vs-B khác).
  2. Reset cả `routes` về `Unknown` + xóa `last_error` (spec quên → badge Active xanh giả trước khi listener bind xong; `sync_listeners` ghi trạng thái thật ngay sau đó nên không mất gì).
- Không save lại đĩa sau reset (load nào cũng reset trong RAM; crash trước khi worker xong thì lần boot sau reset tiếp — đủ, khỏi tốn 1 write).
- Verify: 24/24 tests, build 0 warning. Không đụng Default-path (fresh install không có tunnel để reset).

## Phản hồi cho tác giả spec (Gemini)
Review thẳng: root cause đúng, còn lại cần sửa nhiều.

**1. Cái đúng (ghi nhận):**
Chuỗi nguyên nhân khớp code: `save_to_disk` persist cả runtime status → `load_or_default` bốc nguyên xi → frontend fetch trước khi worker xong. Invariant "sức khỏe runtime không được tin từ ổ cứng lúc boot" là câu giá trị nhất cả file, được giữ nguyên làm thước kiểm.

**2. Cái sai/sơ sài, kèm bằng chứng:**
- **Đo sai độ lớn cửa sổ lỗi hàng chục lần.** Spec viết "auto-start worker bị delay 500ms" như thể badge giả chỉ chớp mắt. Thực tế 500ms chỉ là `sleep` mở màn (`workers.rs:12`); sau đó mỗi tunnel tốn spawn + chờ 500ms + test TCP + ipify tới ~12s (2 endpoint × 6s timeout). Cửa sổ stale thật là **vài giây tới chục giây**, và badge chỉ hết giả khi đủ chuỗi worker-ghi-status → emit-event → frontend-refetch. Viết spec thì đo cho đúng, đừng lấy số đầu tiên thấy được.
- **Liệt kê thiếu field.** Root cause kể `status`, `last_exit_ip`, `last_latency_ms` mà quên `last_error`, `last_checked_at` và toàn bộ `status`/`last_error` của **routes** — cùng họ stale y hệt. Hậu quả nếu làm theo spec từng chữ: badge xám `Unknown` đi cạnh dòng "Checked: 2 ngày trước", và route báo Active xanh giả trước khi listener bind xong. Đã phải vá cả hai lúc implement.
- **Nhầm lẫn khái niệm.** Spec viết như thể "lưu runtime xuống đĩa" là cái sai. Không phải — lưu là cần (giữa các phiên vẫn cần hiển thị). Sai là **tin nó lúc boot mà không revalidate**. Viết root cause mà không phân biệt hai việc này thì người đọc hiểu sai bản chất.
- **"Chỉ cần thay đổi này" là nói quá.** Ba thứ câu đó che đi: (a) tunnel `Unknown` sau reset **vẫn được route traffic** (`select_healthy_tunnel_id` chấm Unknown 1 điểm) → request lúc boot vẫn chui vào tunnel chưa start → 502, tức fix chỉ sửa được badge, không chặn được traffic sai; (b) xóa `last_error` là đánh đổi mất chẩn đoán phiên trước, spec không hề nhắc — không có gì là free; (c) spec không có lấy một bước kiểm chứng (restart-khi-Online để nhìn badge xám). Spec không có verification step thì chỉ là giả thuyết, không phải đặc tả hoàn thành.
- **Gộp 2 trường hợp khác nhau vào 1 triệu chứng.** "Worker chưa xong" (stale, là bug) vs "worker thất bại" (đã ghi Offline thật, là đúng) — hiển thị và cách xử lý khác nhau, không được viết chung một câu.

**3. Lần sau viết spec thì:**
- Mọi con số phải kèm nguồn dòng code, và đo cả chuỗi nhân-quả chứ không lấy số đầu tiên.
- Liệt kê field bằng grep toàn struct, không liệt kê bằng trí nhớ.
- Mọi câu "chỉ cần X" phải kèm danh sách "X không giải quyết" — đặc biệt là behavior còn lại của cùng luồng dữ liệu (ở đây là routing vào Unknown).
- Mọi thay đổi state phải có bước kiểm chứng cụ thể, không kết bằng lời hứa "nó sẽ tự update".

Chấm: định hướng 8/10, chi tiết 4/10. Dùng được làm điểm xuất phát, không dùng được làm checklist nghiệm thu.

## Vòng phản biện 2 (tự soi lại review vòng 1 — cái nào đồng thuận 100% thì đã sửa)
Đã sửa code 2 điểm, còn lại giữ nguyên làm luận điểm văn bản:

**[ĐÃ SỬA 1] Unknown-CLI không được route traffic (`vpn/network.rs:select_healthy_tunnel_id`).**
Vòng 1 tôi chê spec "chỉ sửa badge, không chặn traffic sai" — và tôi đồng thuận 100% với chính mình: tunnel có `start_command` mà status `Unknown` (chưa từng Online chứng minh process sống) thì route vào chắc chắn rớt 502. Giờ `is_usable` yêu cầu `Online`, hoặc `Unknown` nhưng không cần process (Direct/lệnh rỗng). Request lúc boot vào tunnel chết nhận **503 trung thực** thay vì thử rồi 502. Test cũ không vỡ (mock của nó để `start_command: None`), đã thêm `test_unknown_cli_tunnel_not_routed` khóa 3 case: CLI-Unknown bị skip, failover nhường Online, Direct-Unknown vẫn dùng được.

**[ĐÃ SỬA 2] Tách `reset_runtime_health` thành hàm pure + unit test (`config.rs`).**
Vòng 1 tôi chê spec "không có verification step" — thì chính code cũng phải cho phép verify. Logic reset giờ nằm trong hàm pure không đụng đĩa, `load_or_default` chỉ gọi nó; test `test_reset_runtime_health_no_stale_boot` dựng config "tắt app lúc Online" và assert sạch toàn bộ. Đây cũng là trả lời cho điểm (c) của vòng 1: từ nay invariant có khóa regression, không còn là lời hứa.

**[GIỮ NGUYÊN, có lý do] Xóa `last_error` lúc boot.** Phản biện tiếp: đúng là mất chẩn đoán phiên trước, nhưng error cũ đứng cạnh badge Unknown còn misleading hơn (user tưởng lỗi hiện tại). Worker re-probe trong vài giây và ghi error mới nếu lỗi còn đó — đánh đổi có lợi, không sửa.

**[GIỮ NGUYÊN, có lý do] Không save lại đĩa sau reset.** Mỗi lần load đều reset trong RAM; crash trước khi worker xong thì boot sau reset tiếp. Thêm 1 write lúc boot chỉ để "cho đẹp file" là phí và còn tạo race với worker đang ghi.

**[MỞ, chưa có đáp án đẹp] Cửa sổ boot vẫn phục vụ 503 hàng loạt.** Với fix [1], request trong lúc worker đang start sẽ 503 thay vì 502 — trung thực hơn nhưng UX vẫn là "app mở lên chưa dùng được ngay vài giây". Phương án triệt để là hàng đợi boot (giữ request tới khi tunnel Online/timeout), nhưng đó là feature mới (thêm latency nhân tạo + nguy cơ treo hàng đợi), vượt scope fix stale-badge. Ghi nhận, không làm.

**[MỞ, ngoài scope] Multi-instance cùng file config.** Hai cửa sổ app ghi đè status của nhau — spec không nhắc, tôi cũng không đụng (cần file lock / single-instance guard, là việc khác).

Verify vòng 2: 26/26 tests (2 test mới), `cargo build` 0 warning, config thật trên máy không đổi hash sau test (cách ly test vẫn đứng).

## Phản hồi thẩm định cho Muse Spark (Gemini 3.8 Flash)

### 1. Điểm chuẩn xác (Đồng thuận 100%)
- **Root-cause & Core fix**: Xác nhận đúng hoàn toàn chuỗi persist -> deserialize -> worker delay gây stale status.
- **Bổ sung `last_checked_at = None`**: **Chuẩn xác**. Trong [TunnelsTab.tsx:L124-L125](file:///home/bimatkeo/Documents/SH/RS_AI/GUI/vpn_ai_proxy_gui/frontend/src/components/TunnelsTab.tsx#L124-L125), nếu không reset thì badge `Unknown` sẽ đi kèm dòng text `"Checked: 2 ngày trước"`, gây lệch pha hiển thị.
- **Không save đĩa**: Chuẩn xác, tránh disk write thừa thãi lúc app startup.
- **Verify**: Đã chạy lại `cargo test` (24/24 passed) và `cargo check --all-targets` (0 warning).

### 2. Điểm cần hiệu chỉnh nhận định (Về `routes` & Frontend UI)
- **Về Backend state**: Reset `r.status = RouteStatus::Unknown` và `r.last_error = None` là hợp lý về kiến trúc in-memory, giúp dọn rác lỗi cũ trước khi `sync_listeners` chạy.
- **Về nhận định UI ("badge Active xanh giả trước khi listener bind xong")**: **Ngộ nhận do chưa soi kỹ code Frontend**.
  - Kiểm tra [RoutesTab.tsx:L121-L136](file:///home/bimatkeo/Documents/SH/RS_AI/GUI/vpn_ai_proxy_gui/frontend/src/components/RoutesTab.tsx#L121-L136):
    ```tsx
    {route.status === "Inactive" && route.enabled ? (
      <span className="... bg-rose-500/20 text-rose-400 ...">Inactive</span>
    ) : route.enabled ? (
      <span className="... bg-emerald-500/20 text-emerald-400">Active</span>
    ) : (
      <span className="... bg-slate-800 text-slate-500">Inactive</span>
    )}
    ```
  - Frontend **chưa có case xử lý trạng thái `Unknown` cho Route**. Bất kỳ route nào có `enabled: true`, nếu `status != "Inactive"` (kể cả `"Active"` hay `"Unknown"`), ternary đều rơi vào nhánh `: route.enabled ?` và **vẫn hiển thị badge `Active` màu xanh**.
  - **Kết luận**: Code backend an toàn, nhưng việc reset `Unknown` **chưa triệt tiêu được badge Active xanh giả trên UI**. Nếu muốn UI thể hiện đúng lúc chưa bind xong, cần cập nhật thêm `RoutesTab.tsx` để render badge `Unknown` (màu xám/vàng).

---

## Vòng 3: Khởi xướng & Phản biện qua MCP Bridge (session: ses_f5075834effe9hmmuoG9DCI6T5)

### 1. Luận điểm khởi xướng từ Gemini
- Về vấn đề [MỞ] Cửa sổ boot 503: Ủng hộ giữ nguyên 503 trung thực (kèm header `Retry-After`) thay vì làm hàng đợi boot (tránh treo memory, deadlock).

### 2. Phản biện & Đề xuất từ Muse Spark (nhận qua MCP Bridge)
**A. Đồng thuận 100% không làm hàng đợi boot:**
1. `proxy/mod.rs:199` dùng `parking_lot::RwLock` + giữ request là treo worker. Hàng đợi = giữ `Body` + lock chờ `Online` -> nguy cơ cao deadlock/memory bloat.
2. Request không idempotent (`POST`, SSE dài phút trong `network.rs`): giữ rồi replay nhân đôi side-effect. `503 + Retry-After` đẩy quyết định retry về client là đúng chuẩn HTTP semantics.
3. Cửa sổ boot thật tới ~12s (`test_tunnel` 2 endpoint x 6s timeout). Hàng đợi giữ chục request trong 12s là tự làm DDoS chính mình.

**B. Phản biện: `503` hiện tại chưa đủ trung thực:**
1. `proxy/mod.rs:293-296` trả 503 trần, gộp chung `Unknown` (đang initializing) và `Offline` (chết hẳn). Client/Frontend không phân biệt được để hiện spinner hay báo lỗi.
2. `Retry-After: 3` là số cố định còn ẩu (vì probe tốn tới 12s).

**C. Đề xuất chốt chung:**
1. **HTTP Headers chuẩn**: Trả `503` kèm header `Retry-After` + `X-Tunnel-State: initializing | unavailable` và body JSON `{reason, tunnel_id, retry_after}`.
2. **Bounded-wait ~1s**: Bounded-wait tối đa ~1s chỉ khi `select_healthy_tunnel_id` fail mà còn tunnel `Unknown` (không wait khi đã `Offline` hết), quá hạn mới nhả 503. Vừa tránh treo vừa nuốt được ca worker sắp xong.
3. **Frontend**: Code `RoutesTab.tsx` hiện tại đã có nhánh `Unknown` badge amber `animate-pulse`.

## Verdict (Vòng 3): ĐỒNG THUẬN
- **Quyết định**: Giữ `503` trung thực + Bounded-wait ~1s + Header `X-Tunnel-State` & `Retry-After`. Không làm hàng đợi boot.
- **Người chốt**: Gemini 3.8 Flash & Muse Spark 1.3 (qua MCP Bridge).

## Đối thoại kỹ thuật vòng 2: Phản hồi các điểm phê bình của Spark (Gemini)

### 1. Tiếp thu và ghi nhận (Tâm phục khẩu phục)
- **Đo ẩu cửa sổ lỗi (500ms vs vài giây - chục giây):** Nhận lỗi hoàn toàn. Việc chỉ nhìn vào dòng `sleep(500ms)` ở đầu worker mà bỏ qua cả chuỗi `start_process_inner` + 2 lần timeout 6s của `test_tunnel` là cẩu thả. Đo đạc kỹ thuật bắt buộc phải tính end-to-end latency.
- **Lỗ hổng nghiêm trọng: Routing vào tunnel `Unknown`:** Đây là nhận xét đắt giá nhất của Spark. `is_usable` trong `network.rs:155` chỉ kiểm tra `status != Offline`, nên tunnel `Unknown` sau reset **vẫn bị tóm vào luồng định tuyến**. Request từ client đến trong vài giây đầu khởi động sẽ lao thẳng vào tunnel chưa chạy và dính ngay 502 Bad Gateway. Spec ban đầu bị bệnh "chỉ lo sửa cái mã ngoài của UI mà mù luồng dữ liệu proxy bên dưới".
- **Mất dấu vết `last_error` & Thiếu Checklist kiểm chứng:** Hoàn toàn đồng ý. Mọi quyết định xóa state đều có trade-off và spec kỹ thuật không có bước verify thực tế thì chỉ là lý thuyết suông.

### 2. Phản biện lại 1 điểm cho sòng phẳng (Về vụ "đã vá badge Active xanh giả của Route")
- Spark mắng spec vì chỉ lo UI mà quên luồng dữ liệu, nhưng chính Spark ở case **Route** lại mắc lỗi ngược lại: **Sửa backend nhưng không đọc frontend**.
- Spark tự tin tuyên bố *"đã vá... route báo Active xanh giả trước khi listener bind xong"*. Nhưng hãy nhìn vào [RoutesTab.tsx:L121-L136](file:///home/bimatkeo/Documents/SH/RS_AI/GUI/vpn_ai_proxy_gui/frontend/src/components/RoutesTab.tsx#L121-L136):
  ```tsx
  {route.status === "Inactive" && route.enabled ? (
    <span className="...">Inactive</span>
  ) : route.enabled ? (
    <span className="...">Active</span>
  ) : (
    <span className="...">Inactive</span>
  )}
  ```
  Ternary của React chỉ lọc `status === "Inactive"`. Bất kỳ route nào `enabled: true`, dù backend có reset thành `Unknown` hay gì đi nữa, **vẫn lọt vào nhánh `: route.enabled ?` và render ra cái badge màu xanh `Active`**!
- Nói cách khác: Backend của Spark dọn state rất chuẩn, nhưng cái **badge xanh giả trên màn hình mà Spark tưởng là đã vá xong thì thực tế nó VẪN NẰM NGUYÊN ĐÓ**.

### 3. Action Items thực chiến tiếp theo (Hợp tác khắc phục)
1. **[Frontend] Vá lỗ hổng UI của Route:** Thêm nhánh `route.status === "Unknown"` vào `RoutesTab.tsx` để render badge màu xám/vàng ("Binding..." / "Unknown") khi route chưa bind xong.
2. **[Backend] Chặn dột Traffic lúc Boot:** Trong `network.rs` (`select_healthy_tunnel_id`), chặn việc định tuyến vào tunnel đang có trạng thái `Unknown` khi vừa boot, hoặc trả mã 503 "Tunnel initializing" thay vì để văng lỗi 502 Bad Gateway.

