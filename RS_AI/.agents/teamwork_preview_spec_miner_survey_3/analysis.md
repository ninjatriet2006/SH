# Comprehensive Feature & Specification Mining Analysis Report

**Project**: Filen GUI UI Redesign & Specification (filen_gui)  
**Author**: Specification & Feature Miner  
**Date**: 2026-08-05  
**Target Path**: `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/`  
**Rule Compliance**: `I am stupid google products` prefix active; zero code modifications performed; full IDE tools usage; ID Linking localization rules included.

---

## 1. Executive Summary

This report presents a complete specification and feature survey for the `filen_gui` project based on `ORIGINAL_REQUEST.md` and the existing Rust codebase (`main.rs`, `operations.rs`, `transfer.rs`). 

The objective is to produce a blueprint specification for a **Neon-themed GUI** that synthesizes legacy desktop/TUI capabilities with modern dual-pane UX patterns. The design artifact will reside entirely inside a `docs/` directory located adjacent to `src/` (`apps_gui/filen_gui/docs/`).

---

## 2. Requirement Mapping (R1, R2, R3) & Acceptance Criteria

### Requirement Summary
- **R1. Neon Aesthetics & Detailed Component Specs**: Define exact hex color codes, glow shadows, layout grids, typography, hover/active/focused/disabled visual states, and component behaviors.
- **R2. Legacy & Modern Feature Synthesis (Dual Flow & Branching)**: Integrate dual-pane explorer, Nemo multi-selection, drag-and-drop between panes, clipboard operations, quick account switching, 2FA modal flow, background transfer queue, sync pairs, recents, and multi-protocol servers (WebDAV/S3/Mount FUSE).
- **R3. Documentation Architecture (`docs/` Structure)**: Create a standalone design document folder `docs/` containing screen branching logic (user flow) and UI state data models without direct application implementation code.

### Acceptance Criteria Verification Table
| Criteria ID | Description | Status / Plan |
|-------------|-------------|---------------|
| AC-1 | Existence of `docs/` directory next to `src/` (`apps_gui/filen_gui/docs/`) | Verified layout rule |
| AC-2 | Complete Markdown design specification file in `docs/` | Specified structure in analysis |
| AC-3 | Precise hex color palette, dimensions, typography, and glow effects | Defined in Section 6 |
| AC-4 | Complete screen branching & user flow state machine diagram | Defined in Section 8 |
| AC-5 | High level of detail enabling another AI developer to code without clarification | Detailed in Sections 5-9 |

---

## 3. Features Discovered

| # | Category | Feature | Description | Inputs | Outputs | Error Behavior | Discovered Via |
|---|----------|---------|-------------|--------|---------|----------------|----------------|
| 1 | Navigation | Dual Explorer Panes | Side-by-side file navigation supporting local disk & Filen cloud independently | Click, keyboard, mode toggle | Rendered file list (name, size, mod_time) | Fallback to home/root on invalid path | `main.rs:21-76`, `395-404` |
| 2 | Navigation | Pane Mode Switcher | Toggle each pane between Local (🖥️) and Cloud (☁️) mode | Mode button click | Pane reload, breadcrumb reset | Prompt login if switching to Cloud without active session | `main.rs:3252-3273` |
| 3 | Navigation | Smart Breadcrumbs | Interactive path bar with clickable segments and `…` truncation | Breadcrumb segment click | Instant directory jump | Ignore click if already on target path | `main.rs:1334-1366`, `3502-3519` |
| 4 | Selection | Nemo Multi-Select | Single-click, Ctrl+click (toggle), Shift+click (range from anchor) | Mouse clicks + Ctrl/Shift modifiers | Updated selection vector (`selected`) | Clear selection if item deleted or pane mode changed | `main.rs:1717-1752` |
| 5 | Drag & Drop | Inter-Pane Drag-and-Drop | Drag items from source pane to drop on target pane for copy/move | Mouse drag & release, Shift key | Enqueued transfer operation | Highlight target pane; block drop if src == dst path | `main.rs:852-886`, `1490-1509` |
| 6 | Clipboard | Internal Copy/Cut/Paste | Ctrl+C (copy), Ctrl+X (cut), Ctrl+V (paste) across panes | Keyboard shortcuts | Clipboard state & transfer enqueue | Error log if pasting to identical source path | `main.rs:771-848` |
| 7 | Authentication | Stored Accounts & Quick Switch | Store credentials in `accounts.json` with 0600 permissions for 1-click login | Account row click / quick login button | Session switch, cloud re-list | Log error if saved password invalid | `main.rs:1028-1069`, `operations.rs:36-84` |
| 8 | Authentication | 2FA Challenge Workflow | Modal transitions to 2FA prompt when account requires verification | 2FA code input string | Authenticated session token | Show inline error message on invalid code | `main.rs:1133-1166`, `2830-2845` |
| 9 | Operations | Context Menu File Ops | Right-click menu for Mkdir, Rename, Delete, Favorite, Cat, Link, Copy Path | Right-click on item / empty space | Action modal / async execution | Inline error banner in status bar | `main.rs:1619-1660` |
| 10 | Operations | File Content Viewer (Cat) | Async file text fetch with scrollable view & copy button | Double click / View context menu | Text viewer modal | Display error string in log if binary/unreadable | `main.rs:2516-2537`, `operations.rs:644-653` |
| 11 | Operations | Public Share Link | Generate Filen public share link & copy to clipboard | Context menu "Copy Link" | Link modal + clipboard update | Display error log if non-cloud file | `main.rs:2539-2557`, `operations.rs:797-800` |
| 12 | Transfers | Async Transfer Queue | Background runner for Upload, Download, Copy, Move with TTY script parsing | Enqueue events | Real-time progress bar, speed, ETA | Retry/Error state with cancel button | `transfer.rs:1-399` |
| 13 | Views | Recents View | Displays recently accessed cloud files from `Operations::recents` | Sidebar click "Gần đây" | Sorted recent file table | Prompt login if cloud session inactive | `main.rs:1790-1870` |
| 14 | Views | Sync Pair Runner | Lists sync pairs from `syncPairs.json` with one-click execution | Sidebar click "Đồng bộ" -> "▶ Chạy" | Async sync execution | Display sync error message banner | `main.rs:1872-1970` |
| 15 | Servers | Server Control Center | WebDAV, S3, and Mount FUSE child process launcher with live log tail | Port/User/Pass/HTTPS inputs + Start/Stop | Running badge, active child handle | Capture stdio logs into UI scroll area | `main.rs:1972-2198` |
| 16 | System | ID Linking Localization | Dynamic language ID properties (`lang_id`, `lang_type`) on UI widgets | UI lifecycle init | Real-time dynamic string translation | Fallback to default key if ID missing | User Rule requirement |

---

## 4. Edge Cases

| # | Feature | Input | Observed Behavior |
|---|---------|-------|-------------------|
| 1 | Clipboard Paste | Paste (Ctrl+V) when source pane mode changed to Cloud after copy | App warns user that source mode changed, pastes using current mode |
| 2 | Drag & Drop | Drop items onto same pane or identical target path | Dropping is ignored; status log shows "📍 Target equals source" |
| 3 | File Delete | Delete multiple items with "Xóa vĩnh viễn" (no_trash) checked | CLI issues 2 sequential confirmation prompts; interactive wrapper handles both |
| 4 | Binary View | Executing `cat` view on large binary file | Truncates text output or displays raw string; provides copy button |
| 5 | Cloud List | Open Cloud pane without logged-in account | Displays empty list with status "Chưa đăng nhập tài khoản Cloud" |
| 6 | Column Resize | Drag header handle beyond panel bounds | Clamped between min 50px and max 400px; reverse delta calculation handles right-alignment |
| 7 | TTY Progress | CLI upload/download without pseudo-TTY | `script -qec` wrapper forces TTY so carriage-return `\r` progress lines are captured |
| 8 | 2FA Login | User submits credentials for account with 2FA enabled | Login call returns `2FA_REQUIRED`; modal updates to 2FA input state automatically |

---

## 5. Comprehensive Feature Inventory & Requirement Mapping

| Feature ID | Feature Name | Description | R1 (Neon) | R2 (Hybrid UX) | R3 (Docs Spec) | Acceptance Criteria Mapping |
|------------|--------------|-------------|-----------|----------------|----------------|-----------------------------|
| F-01 | Dual-Pane Explorer | Side-by-side pane container with dynamic resizers | Glowing border on active pane | Combines Local & Cloud modes | Full state data model in spec | AC-3, AC-4, AC-5 |
| F-02 | Mode Selector | Local/Cloud switcher per pane | Cyan/Purple neon glyph toggle | Seamless switching | Screen branch transition | AC-3, AC-4 |
| F-03 | Smart Breadcrumbs | Segmented navigation bar | Glowing path hover effects | Truncated breadcrumbs (`…`) | Navigation state structure | AC-3, AC-4 |
| F-04 | Nemo Selection Model | Multi-select with Anchor tracking | Highlighted row background | Ctrl/Shift Nemo logic | Selection state vector spec | AC-4, AC-5 |
| F-05 | Inter-Pane Drag-Drop | Drag items across panes | Neon blue overlay + glowing drop zone | Copy default, Shift=Move | Drag state & event flow | AC-3, AC-4, AC-5 |
| F-06 | Internal Clipboard | Copy/Cut/Paste shortcuts | Notification banner glow | Clipboard buffer tracking | Clipboard data structure | AC-4, AC-5 |
| F-07 | Stored Accounts | Quick account switcher | Status indicators (● active / ○ saved) | Multi-account stored credentials | Account state schema | AC-4, AC-5 |
| F-08 | 2FA Login Flow | Two-step modal auth | Glowing error & input box focus | Interactive 2FA prompt | Auth state machine diagram | AC-4, AC-5 |
| F-09 | File Context Menu | Popup action menu | Dark glassmorphism floating panel | Full file action set | Context menu action tree | AC-3, AC-4, AC-5 |
| F-10 | File Content Viewer | Scrollable text preview modal | Monospace font with cyan header | Quick inspect & copy text | Modal state specification | AC-3, AC-4 |
| F-11 | Share Link Generator | Public link creator | Glowing URL container | One-click copy link | Link data structure | AC-3, AC-4 |
| F-12 | Transfer Manager | Collapsible bottom progress panel | Neon cyan progress bar & green status | Concurrent queue runner | Transfer state & progress model | AC-3, AC-4, AC-5 |
| F-13 | Recents View | Cloud recent file inspector | Accent row highlights | Quick view/link actions | Recents view state spec | AC-4, AC-5 |
| F-14 | Sync Pair Manager | Local-Remote pair manager | Status badge indicators | One-click sync trigger | Sync pair schema | AC-4, AC-5 |
| F-15 | Servers Center | WebDAV/S3/FUSE controller | Live green/grey running badges | Embedded log streamer | Server process state model | AC-3, AC-4, AC-5 |
| F-16 | UI Localization | Langs by ID linking | N/A (Infrastructure) | Real-time dynamic language switch | Lang ID data dictionary | AC-5 |

---

## 6. Neon Theme Aesthetics & Component Design Specification (R1)

### Color Palette (Neon Cyberpunk Theme)
- **Background Base**: `#0F121A` (Deep Obsidian Dark)
- **Active Pane Fill**: `#161C28` (Dark Slate Blue)
- **Primary Accent (Cyan Glow)**: `#5E9CFF` / `#00F0FF` (Glow: `0 0 12px rgba(0,240,255,0.6)`)
- **Secondary Accent (Purple Glow)**: `#A855F7` / `#D8B4FE` (Glow: `0 0 10px rgba(168,85,247,0.5)`)
- **Success Glow (Cyber Emerald)**: `#5AC878` / `#00FF99` (Glow: `0 0 8px rgba(0,255,153,0.5)`)
- **Error Glow (Neon Crimson)**: `#FF5A5A` / `#FF0055` (Glow: `0 0 10px rgba(255,0,85,0.6)`)
- **Warning Glow (Cyber Amber)**: `#F0C85A` / `#FFE600` (Glow: `0 0 8px rgba(255,230,0,0.5)`)
- **Text Main**: `#E6E9EE` (Bright Platinum)
- **Text Sub/Muted**: `#96A0AF` (Cool Grey)
- **Header Background**: `#212732` (Elevated Panel Header)
- **Hover Row Fill**: `#262E3C` (Subtle Highlight)

### Typography & Component Layout
- **Font Family**: Proportional Sans (System / Noto Sans / DejaVu Sans), Monospace (Noto Sans Mono)
- **Headings**: 18px Bold with Cyan Text Shadow
- **Item Rows**: Height 26px, padding 6px, rounded corners 4.0px
- **Pane Outer Border**: 1.0px `#455060` (Inactive), 1.5px `#5E9CFF` with 12px Cyan Glow (Active)
- **Drop Target Overlay**: 2.0px `#5E9CFF` dashed outline with centered semi-transparent notification badge

---

## 7. UI State Data Architecture & Data Models (R3)

```rust
// Proposed UI State Architecture for Design Document
struct AppState {
    panes: [PaneState; 2],
    active_pane: usize,
    account: AccountState,
    login_form: LoginFormState,
    view_mode: MainView,
    modal: Option<ModalState>,
    recents: RecentsState,
    sync: SyncState,
    servers: ServersState,
    transfer: TransferManagerState,
    clipboard: Option<ClipboardState>,
    drag_drop: Option<DragDropState>,
    system_log: Vec<LogEntry>,
}

struct PaneState {
    mode: PaneMode, // Local | Cloud
    path: String,
    items: Vec<FileItem>,
    filter_query: String,
    selected_items: Vec<String>,
    anchor_item: Option<String>,
    navigation_back_stack: Vec<String>,
    navigation_forward_stack: Vec<String>,
    column_widths: [f32; 3], // Size, Type, ModTime
}

struct AccountState {
    active_email: Option<String>,
    stored_accounts: Vec<StoredAccount>,
    storage_used_bytes: u64,
    storage_max_bytes: u64,
    is_busy: bool,
}

struct LoginFormState {
    is_open: bool,
    email_input: String,
    password_input: String,
    keep_logged_in: bool,
    is_2fa_required: bool,
    twofa_code_input: String,
    error_message: Option<String>,
}

struct ClipboardState {
    source_pane: usize,
    source_mode: PaneMode,
    source_path: String,
    items: Vec<String>,
    is_cut_operation: bool,
}
```

---

## 8. Screen Branching Rules & User Flow Diagrams (R2, R3)

```
                       +-------------------------+
                       |      App Launch         |
                       +------------+------------+
                                    |
                                    v
                       +-------------------------+
                       |   Check Active Session  |
                       +------+-----------+------+
                              |           |
                   Session OK |           | No Session
                              v           v
             +------------------+       +------------------+
             | Main Explorer    |       | Login Modal      |
             | (Dual Pane)      |       | (Email/Password) |
             +--------+---------+       +--------+---------+
                      |                          |
                      |                          | Submit Credentials
                      |                          v
                      |                 +------------------+
                      |                 | Check 2FA        |
                      |                 +---+----------+---+
                      |             2FA Req |          | Auth OK
                      |                     v          v
                      |             +---------------+  |
                      |             | 2FA Prompt    +--+
                      |             +---------------+
                      v
     +-------------------------------------------------------+
     |                    Main Explorer                      |
     |  Pane 0 (Local/Cloud) <-----> Pane 1 (Local/Cloud)  |
     +---+--------------+--------------+---------------+-----+
         |              |              |               |
         v              v              v               v
  +--------------+ +----------+ +-------------+ +---------------+
  | Recents View | | Sync View| | Server View | | Modal Actions |
  | (Operations) | |(syncPairs| |(WebDAV/S3/  | |(Mkdir/Rename/ |
  +--------------+ +----------+ | FUSE Mount) | | Delete/Cat)   |
                                +-------------+ +---------------+
```

---

## 9. File Layout Rules (`docs/` Structure)

As specified in Requirement R3, design documents must be stored in a dedicated `docs/` folder located at the root of `apps_gui/filen_gui/`, next to `src/`.

### Directory Tree Plan
```
apps_gui/filen_gui/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── operations.rs
│   └── transfer.rs
└── docs/
    ├── UI_DESIGN_SPEC.md       # Primary Neon GUI Specification & Component System
    ├── USER_FLOW_BRANCHING.md   # Screen Branching Diagrams & Logic Rules
    ├── UI_STATE_DATA_MODEL.md  # UI State Structures & Data Binding Specification
    └── FEATURE_INVENTORY.md    # Complete Feature & Requirement Traceability Matrix
```

---

## 10. Summary & Next Steps for Team

1. **Spec Miner Handoff**: Deliver `analysis.md` and `handoff.md` to working directory.
2. **Parent Orchestrator Notification**: Send formal completion message.
3. **Downstream Implementation Team**: Read specification files from `docs/` and build neon UI components without ambiguity.
