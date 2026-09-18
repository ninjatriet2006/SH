# Plan: Per-Endpoint Fingerprint (vpn_ai_proxy_gui)

Goal: mỗi endpoint (RouteRule) chọn 1 fingerprint profile từ pool chung; bỏ dùng chung 1 active global cho mọi request.
Pairing: Dev=Muse Spark / Check=SeekAI GLM flash (phase chẵn mặc định).

## Contract
- `RouteRule.fingerprint_index: Option<usize>` (None = dùng global default). Serde default None → tương thích config cũ.
- `GatewayConfig::get_route_fingerprint(route) -> FingerprintProfile`: Some(i)+pool non-empty → pool[i%len], else fallback `get_active_fingerprint()`.
- `handle_route_request`: lấy profile theo route, không gọi global trực tiếp.
- `handle_error_session_rotation`: rotate per-route (route có index → advance index đó; None → advance global như cũ).
- Global pool + active_index giữ làm thư viện + default; FingerprintTab ghi chú rõ.
- Frontend: `types.ts` RouteRule + `Modals.tsx` select + `RoutesTab.tsx` badge/quick-select.

## Tasks
- [ ] T1 Dev: backend config.rs + proxy/mod.rs + rotation.rs + migration/clamp
- [ ] T2 Dev: frontend types.ts + Modals.tsx + RoutesTab.tsx + FingerprintTab note
- [ ] T3 Dev: cập nhật core_tests.rs (RouteRule literal + rotation per-route test)
- [ ] Gate Test: TARGETED+CONTRACT `cargo test -p vpn_ai_proxy_gui_lib` + clippy
- [ ] Gate Check: DEEP, file:line + trace edge, verify độc lập

## Current status
- Active: none; owner: Lead; depends: none
- Passed: core_tests 31/31 + tsc EXIT 0; clippy 26 pre-existing warnings
- Open findings: none (Check backend APPROVE + frontend APPROVE)
- Next gate: done — chưa commit theo quy tắc
