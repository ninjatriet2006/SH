# Plan: Criteria & LoginMethod refactor + versioned migration (account_hub_gui)

## Goal
Thay 4 cờ cứng + custom criteria của Website bằng 2 entity quản lý độc lập
(Criterion, LoginMethod) gán qua dropdown; `category` → `tags[]`;
backup + migrate schema v1→v2 lúc khởi động, tương thích ngược 3 phiên bản.

## Decisions (user-confirmed)
- 1A: `category: String` → `tags: string[]` (migrate: category cũ → 1 tag).
- 2B: Xóa `can_cheat_account/requires_kyc/requires_proxy` khỏi model.
  Hệ quả: bỏ checkbox "Hỗ trợ Cheat" trong EmailWebsitesModal;
  panel Dashboard "Multi-Account / Cheat" → tổng quan tiêu chí.
- 3B (default, user chưa phản đối): xóa `custom_criteria`, migrate entry cũ → tag `"label: value"`.
- Q4: `Criterion{id,name,description?,created_at}`, `LoginMethod` y hệt;
  CRUD đầy đủ 2 trang mới; Website có `criterion_ids[]`, `login_method_ids[]`
  gán bằng dropdown multi-select trong form edit.
- `has_daily_checkin` GIỮ trong model + form edit (điều khiển nút điểm danh),
  ẨN khỏi mọi badge hiển thị.

## Contract (schema v2, CURRENT_SCHEMA_VERSION = 2)
- `AppDatabase += { criteria[], login_methods[], schema_version: u32 (default 1) }`.
- `Website`: `-category -can_cheat_account -requires_kyc -requires_proxy
  -custom_criteria`, `+tags[] +criterion_ids[] +login_method_ids[]`,
  giữ `has_daily_checkin`.
- Loader chấp nhận version trong [CURRENT-2, CURRENT]; version mới hơn/cũ hơn → Err rõ ràng.
- Migrate v1→v2 (backup timestamp `accounts_data.json.bak.YYYYMMDD-HHMMSS` TRƯỚC khi ghi):
  category→tag; custom entry→tag; flag cheat/kyc/proxy → seed criterion
  (`crit-cheat`/"Cheat / Multi-acc", `crit-kyc`/"KYC", `crit-proxy`/"Proxy") + gán
  cho web có flag; `login_method_ids=[]`.
- Commands mới: `save_criterion/delete_criterion/save_login_method/delete_login_method`
  (delete gỡ gán khỏi websites, như delete_email); id prefix `crit-`/`lm-`.

## Tasks
| ID | Work | Owner | Acceptance |
|----|------|-------|------------|
| T1 | models.rs + storage.rs: structs mới, versioned loader, backup, migrate v1→v2, sample data mới | Dev | A1,A2 |
| T2 | lib.rs: `?` giữ nguyên, 4 commands CRUD mới (+unassign khi delete) | Dev | A3 |
| T3 | bridge/types.ts + api.ts + store.ts: types + API + store mới | Dev | A3 |
| T4 | CriteriaPage + LoginMethodsPage + nav/routes (CRUD như EmailsPage) | Dev | A4 |
| T5 | WebsitesPage: form tags + 1 checkbox điểm danh + 2 multi-select; bảng badges mới | Dev | A5 |
| T6 | RegistrationsPage + 2 modals + Dashboard: badges tiêu chí/login, bỏ filter cheat | Dev | A5,A6 |
| T7 | Test migrate v1→v2 (tags, seed criteria + gán, custom→tags, backup tồn tại) | Dev | A7 |
| T8 | Verify: cargo test + tsc + vite build | Tester | A7,A8 |

## Acceptance
- A1: File v1 (không schema_version) load được, thành v2 đúng transform.
- A2: Backup `.bak.TIMESTAMP` tạo trước khi migrate; version ngoài window → Err rõ ràng.
- A3: CRUD criteria/login methods qua IPC hoạt động; delete gỡ gán.
- A4: 2 trang mới CRUD đầy đủ, vào được từ sidebar.
- A5: Không còn tham chiếu `category/can_cheat_account/requires_kyc/requires_proxy/custom_criteria` trong src (trừ code migrate v1).
- A6: Nút điểm danh + filter điểm danh + streak vẫn đúng (test cũ pass).
- A7: `cargo test` pass (test cũ + test migrate).
- A8: `tsc --noEmit` + `vite build` pass.

## Gates
- Test mức CONTRACT: cargo test + tsc + build.
- Check mức DEEP (đổi contract): file:line + bằng chứng, cấm khen suông.

## Current status
- Active: none (done); owner: Lead
- Passed: cargo test 2/2, tsc clean, vite build ok (Tester); 13 edge traced, APPROVE (Reviewer)
- Open findings: none
- Next gate: none — ready for user acceptance
