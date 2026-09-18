# Plan: Rename vpn_ai_proxy_gui → gateway_filter + preset UA AI

Goal: đổi tên dự án thành gateway_filter; thêm preset UA tool AI có thật.
Pairing: Dev=Muse Spark / Check=SeekAI GLM flash.

## Contract
- Rename: dir GUI/gateway_filter (git mv); Cargo package gateway_filter, lib gateway_filter_lib;
  root Cargo.toml member; tauri productName/identifier com.gateway.filter + window title;
  frontend package.json name; env GATEWAY_FILTER_CONFIG_DIR (fallback VPN_AI_PROXY_* cũ);
  ~/.config/gateway_filter + gateway_filter_config.json; load_or_default tự migrate config cũ sang mới.
- Mọi `vpn_ai_proxy_gui_lib::` → `gateway_filter_lib::` (src + tests). Grep toàn repo trừ target/, evidence/, TODO/ cũ.
- Preset mới (FingerprintTab PRESETS + labels, UA khóa nhập như preset cũ):
  codebuddy `CLI/2.137.1 CodeBuddy/2.137.1` + spoof X-IDE-Name/Type CLI, X-Product SaaS, X-Client-Platform web;
  opencode_cli `opencode/1.14.28`; claude_code `claude-cli/2.1.205 (external, cli)`;
  gemini_cli `GeminiCLI/0.10.0 (linux; x64)`; kimi_cli `KimiCLI/1.5`.
- Thứ tự: rename trước, preset sau, cùng 1 Dev.

## Tasks
- [ ] T1 Dev: rename toàn diện + migration config cũ
- [ ] T2 Dev: 5 preset UA AI
- [ ] Gate Test: CONTRACT cargo test -p gateway_filter + tsc; Gate Check: DEEP scoped

## Current status
- Active: none; owner: Lead; depends: none
- Passed: core_tests 31/31 + tsc 0; sót rename 0 (fallback cũ có chủ ý)
- Open findings: none (Check config/disk APPROVE)
- Next gate: done — chưa commit theo quy tắc
