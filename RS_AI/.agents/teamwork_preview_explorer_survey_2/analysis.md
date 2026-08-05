<!-- INTEGRITY NOTES: UI/UX Workflow Analysis Document for filen_GUI Neon UI Design -->
# UI/UX Workflow Analysis & Neon UI System Specification

**Project**: `filen_GUI` (Rust / egui)  
**Author**: UI/UX Workflow Explorer  
**Date**: 2026-08-05  

---

## 1. Executive Summary & Objective

The goal of this analysis is to define a comprehensive UI/UX design blueprint for `filen_GUI` inspired by cyberpunk neon aesthetics, synthesizing the speed and efficiency of the existing Terminal User Interface (`filen_tui`) with the rich interactive capabilities of the modern Graphical User Interface (`apps_gui/filen_gui`).

The output provides exact color specs, glow parameters, component state matrices, detailed logic branches, screen transition flows, and localization architecture (`lang_id` ID Linking mechanism) so that downstream implementers can create the final design document in `docs/` and build the UI without ambiguity.

---

## 2. Old Workflow (TUI) vs New Workflow (GUI) & Superior Hybrid Synthesis (Requirement R2)

### 2.1 Analysis of Old Workflow (`filen_tui`)
- **Navigation Model**: Screen-based switching (`MainMenu` -> `Explorer`, `Account`, `Servers`). Single item selection per pane with `J`/`K` or Up/Down arrow keys.
- **Account Management**: `accounts.yaml` storage with multi-account quick selection modal (`QuickLoginSelect`), 2FA challenge popup (`TwoFAInput`), and automatic background CLI credential checking (`whoami`).
- **File Operations**: Keybinding menu (`SpecialActionsMenu`) triggering `Mkdir`, `Rename`, `Delete`, `ViewFile`, `CopyLink`, `ExportKey` dialogs.
- **Strengths**: High speed, zero mouse dependency, clean modal prompt sequence, simple status messaging.
- **Weaknesses**: Cannot view dual panes simultaneously for visual side-by-side comparison, lack of drag-and-drop, visual representation limited to terminal text styles.

### 2.2 Analysis of Current GUI Draft (`apps_gui/filen_gui`)
- **Navigation Model**: Sidebar + Central view switching (`Explorer`, `Recents`, `Sync`, `Servers`). Dual-pane explorer rendered side-by-side (Left = Pane 0, Right = Pane 1).
- **Interactive Features**: Dynamic breadcrumbs with clickable path segments, internal clipboard (`Ctrl+C`, `Ctrl+X`, `Ctrl+V`), pointer drag-and-drop between panes (with Shift key modifier for Move vs Copy).
- **Background Operations**: Threaded `TransferManager` managing async transfers with progress feedback, multi-process management for WebDAV, S3, and FUSE mount servers.
- **Strengths**: Visual dual-pane layout, drag-and-drop, real-time progress bars, breadcrumb navigation.
- **Weaknesses**: Lacks keyboard-first rapid control, visual aesthetics are plain dark/gray without high contrast or glowing feedback, account quick switcher lacks instant key toggles.

### 2.3 Superior Hybrid Workflow Synthesis (R2)
The hybrid workflow combines the **keyboard agility of TUI** with the **spatial visualization of GUI**:

```
+---------------------------------------------------------------------------------------------------+
| SYNTHESIZED HYBRID WORKFLOW                                                                       |
+---------------------------------------------------------------------------------------------------+
| 1. Dual-Pane Side-by-Side Spatial Awareness (GUI) + Instant Tab/Key Focus Switching (TUI)         |
| 2. Drag & Drop Visual Transfer (GUI) + Hotkey Cut/Copy/Paste with Clear Selection Anchor (TUI/GUI)|
| 3. Interactive Clickable Breadcrumb (GUI) + Quick Search Jump Bar (TUI '/' key focus)             |
| 4. Multi-Account Quick Dropdown in Sidebar (GUI) + Step-by-Step 2FA Modal Flow (TUI)              |
| 5. Context Menu Mouse Actions (GUI) + Global Command Palette / Action Menu (TUI 'M' key)          |
| 6. Real-Time Process Console for WebDAV/S3/Mount (GUI) + Auto-Status Badges (TUI)                 |
+---------------------------------------------------------------------------------------------------+
```

---

## 3. Neon Color Palette & Aesthetic Specifications (Requirement R1)

The UI uses a **Cyberpunk Dark Neon** aesthetic. Primary background is deep obsidian void, surfaces use low-reflectance midnight blue-gray, with high-intensity glowing neon accents for interactive state cues.

### 3.1 Color Palette Specifications Table

| Token Name | Hex Code | Opacity | Role & Usage |
|---|---|---|---|
| `BG_OBSIDIAN` | `#080A10` | 100% | Main application canvas background |
| `SURFACE_PANEL` | `#0E121E` | 100% | Pane container background, card background |
| `SURFACE_GLASS` | `#161C2E` | 85% | Floating modals, tooltips, overlay backgrounds |
| `BORDER_MUTED` | `#222C42` | 100% | Default subtle panel and component borders |
| `NEON_CYAN` | `#00F3FF` | 100% | Primary accent, active pane border, focused inputs |
| `NEON_CYAN_GLOW` | `#00F3FF` | 40% | Outer drop glow for primary highlights |
| `NEON_MAGENTA` | `#FF007F` | 100% | Secondary accent, cut selection, high priority alert |
| `NEON_PURPLE` | `#9D00FF` | 100% | Section headers, Cloud indicator, server state |
| `NEON_EMERALD` | `#00FF87` | 100% | Success status, active account badge, running server |
| `NEON_AMBER` | `#FFB800` | 100% | Transfer in-progress, warning alert, pending sync |
| `NEON_CRIMSON` | `#FF2E63` | 100% | Error message, delete confirm modal, stopped server |
| `TEXT_PRIMARY` | `#F0F4FC` | 100% | Main headings, filenames, primary text |
| `TEXT_SECONDARY` | `#94A3B8` | 100% | Metadata (file size, date), labels, inactive tabs |
| `TEXT_MUTED` | `#475569` | 100% | Placeholder text, disabled labels |

### 3.2 Glow & Visual Effect Specs

```css
/* Primary Active Glow (Cyan) */
.neon-active-border {
  border: 1px solid #00F3FF;
  box-shadow: 0 0 10px rgba(0, 243, 255, 0.4), inset 0 0 5px rgba(0, 243, 255, 0.2);
}

/* Secondary Action Glow (Magenta) */
.neon-magenta-glow {
  border: 1px solid #FF007F;
  box-shadow: 0 0 12px rgba(255, 0, 127, 0.5);
}

/* Hover State Glow Pulse */
.neon-hover-pulse {
  background-color: #1A2338;
  border-color: #00F3FF;
  box-shadow: 0 0 8px rgba(0, 243, 255, 0.3);
  transition: all 0.15s ease-in-out;
}

/* Text Glow Effect for Neon Headers */
.neon-text-cyan {
  color: #00F3FF;
  text-shadow: 0 0 8px rgba(0, 243, 255, 0.6);
}
```

---

## 4. Typography & Hierarchical Font System

| Level | Font Family / Type | Size | Weight | Line Height | Color Token |
|---|---|---|---|---|---|
| **App Title** | Proportional / Header | 18px | Bold (700) | 24px | `NEON_CYAN` + Text Glow |
| **Section Header** | Proportional | 14px | SemiBold (600) | 20px | `NEON_PURPLE` / `TEXT_PRIMARY` |
| **Body / Filename** | Proportional | 13px | Regular (400) | 18px | `TEXT_PRIMARY` |
| **Metadata / Subtext**| Proportional | 11px | Regular (400) | 16px | `TEXT_SECONDARY` |
| **Code / Path / Key** | Monospace | 12px | Regular (400) | 16px | `NEON_CYAN` or `TEXT_PRIMARY` |
| **Badge / Label** | Proportional | 10px | Bold (700) | 14px | `NEON_EMERALD` / `NEON_AMBER` |

---

## 5. Component States & Interactive Feedback Matrix

| Component | State | Background | Border Color | Glow Effect | Text Color | Cursor |
|---|---|---|---|---|---|---|
| **Primary Button** | Normal | `#121A2E` | `#00F3FF` | None | `#00F3FF` | Default |
| | Hover | `#1A2845` | `#00F3FF` | `0 0 10px rgba(0,243,255,0.4)` | `#FFFFFF` | Pointer |
| | Active | `#00F3FF` | `#00F3FF` | `0 0 15px rgba(0,243,255,0.8)` | `#080A10` | Pointer |
| | Disabled | `#0E121E` | `#222C42` | None | `#475569` | Not-Allowed |
| **Explorer Row** | Normal | Transparent| Transparent | None | `#F0F4FC` | Pointer |
| | Hover | `#162035` | Transparent | None | `#00F3FF` | Pointer |
| | Selected | `#1E2D4A` | `#00F3FF` (1px) | `0 0 6px rgba(0,243,255,0.2)` | `#FFFFFF` | Pointer |
| | Cut State | `#2A1628` | `#FF007F` (1px) | `0 0 6px rgba(255,0,127,0.3)` | `#FF80BF` | Pointer |
| **Input Field** | Normal | `#0E1422` | `#222C42` | None | `#F0F4FC` | Text |
| | Focused | `#12192B` | `#00F3FF` | `0 0 8px rgba(0,243,255,0.4)` | `#FFFFFF` | Text |
| | Error | `#1F0E16` | `#FF2E63` | `0 0 8px rgba(255,46,99,0.4)` | `#FF809B` | Text |
| **Pane Container**| Active Focus | `#0E1322` | `#00F3FF` | `0 0 12px rgba(0,243,255,0.3)` | N/A | N/A |
| | Inactive Focus | `#0A0D16` | `#1E2638` | None | N/A | N/A |
| | Drag-Over Target| `#121F38` | `#00FF87` | `0 0 16px rgba(0,255,135,0.5)` | N/A | Copy/Move |

---

## 6. Layout Grid & Structure Specifications

```
+---------------------------------------------------------------------------------------------------+
| TOP BAR: App Logo [Neon Cyan] | Active Account Indicator | Storage Bar | Settings | Lang Toggle  |
+--------------------------+------------------------------------------------------------------------+
| NAVIGATION SIDEBAR       | CENTRAL CONTENT VIEW AREA                                              |
| (Width: 210px)           |                                                                        |
|                          | [View Selector Tabs: Explorer | Recents | Sync | Servers]               |
| [LOCAL PLACES]           | +-----------------------------------+----------------------------------+ |
|  - Home                  | | PANE 0 (LEFT) [Active / Glow Cyan]| PANE 1 (RIGHT) [Inactive]        | |
|  - Desktop               | | Toolbar: [<-][->][^][Home][Refresh]| Toolbar: [<-][->][^][Home][Ref]  | |
|  - Documents             | | Mode: [Local | Cloud Toggle]      | Mode: [Local | Cloud Toggle]     | |
|  - Downloads             | | Path: /home/user/documents        | Path: /cloud/remote/folder       | |
|                          | | Filter: [ Filter... ]             | Filter: [ Filter... ]            | |
| [FILEN CLOUD]            | +-----------------------------------+----------------------------------+ |
|  - Cloud Root            | | Item Table (Headers: Name|Size|Mod)| Item Table (Headers: Name|Size)  | |
|  - Recents               | | [Dir] Projects                    | [Dir] Backups                    | |
|  - Sync Pairs            | | [File] report.pdf                 | [File] dataset.tar.gz            | |
|  - Servers               | |                                   |                                  | |
|                          | +-----------------------------------+----------------------------------+ |
| [ACCOUNTS SECTION]       | | Pane Status: 2 items selected     | Pane Status: Ready               | |
|  - Active Email Badge    | +-----------------------------------+----------------------------------+ |
|  - Quick Switcher List   |                                                                        |
|  - Login / Logout Btns   |                                                                        |
+--------------------------+------------------------------------------------------------------------+
| BOTTOM DOCK: Active Transfers (Count, Speed, Progress Bar) | System Log Terminal Expander       |
+---------------------------------------------------------------------------------------------------+
```

---

## 7. Detailed Logic Branching & Screen Transition Flows (Requirement R2)

### 7.1 Authentication & Account Switch Logic Branch

```
[Start App]
   │
   ├─► Read stored accounts from ~/.config/filen-cli/accounts.yaml
   ├─► Check CLI session via Operations::whoami()
   │      ├─► Active session found ──► Set active_account = email ──► Load storage info ──► Activate Cloud Pane
   │      └─► No active session ────► Set active_account = None ───► Cloud Pane shows "Not Logged In"
   │
   ├─► [User Clicks "Login New"] ──► Open LoginForm Modal
   │      │
   │      ├─► Enter Email & Password ──► Submit
   │      │      │
   │      │      ├─► Login Success ──► Save credential (if keep_logged=true) ──► Close Modal ──► Refresh UI
   │      │      │
   │      │      ├─► 2FA Required Alert ──► Transition Modal to 2FA Step Input
   │      │      │      │
   │      │      │      ├─► Submit 2FA Code ──► Success ──► Save & Refresh
   │      │      │      └─► Invalid 2FA Code ─► Show Crimson Error Text
   │      │      │
   │      │      └─► Login Error (Bad credentials) ──► Show Crimson Error Text in Modal
   │      │
   │      └─► [User Clicks "Cancel"] ──► Close Modal
   │
   └─► [User Clicks Stored Account Quick Switch]
          │
          ├─► If password stored ──► Async re-login call ──► Success ──► Update active_account
          └─► If no password ────► Open LoginForm Modal pre-filled with email
```

### 7.2 File Transfer Logic Branch (Drag-and-Drop & Clipboard)

```
[User Initiates Action: Drag-and-Drop OR Ctrl+C / Ctrl+X -> Ctrl+V]
   │
   ├─► Identify Source Pane (src_pane) and Destination Pane (dst_pane)
   │
   ├─► Validate Destination:
   │      ├─► src_pane == dst_pane AND src_path == dst_path ──► Abort ("Destination same as Source")
   │      └─► Destination valid ──► Continue
   │
   ├─► Check Cloud Authentication:
   │      ├─► (src_mode == Cloud OR dst_mode == Cloud) AND active_account is None
   │      │      └──► Abort with Alert: "Cloud authentication required"
   │      └─► Credentials valid ──► Continue
   │
   ├─► Determine Transfer Type Matrix:
   │      │
   │      ├─► Local -> Local:
   │      │      ├─► Copy (Ctrl+C / Drag) ──► Enqueue fs::copy task
   │      │      └─► Move (Ctrl+X / Shift+Drag) ──► Enqueue fs::rename / move task
   │      │
   │      ├─► Local -> Cloud:
   │      │      ├─► Copy ──► Enqueue Upload task (CLI filen upload)
   │      │      └─► Move ──► Enqueue Upload task + Cleanup source file upon success
   │      │
   │      ├─► Cloud -> Local:
   │      │      ├─► Copy ──► Enqueue Download task (CLI filen download)
   │      │      └─► Move ──► Enqueue Download task + Cleanup cloud file upon success
   │      │
   │      └─► Cloud -> Cloud:
   │             ├─► Copy ──► Operations::cp(src, dst)
   │             └─► Move ──► Operations::mv(src, dst)
   │
   └─► Execution & Real-time Progress Tracking:
          ├─► Add items to TransferManager queue
          ├─► Spawn background worker threads (up to max_concurrent)
          ├─► Send AsyncResult::TransferProgress to channel
          ├─► UI updates transfer dock progress bars & speed metrics
          └─► On completion: Refresh affected explorer panes & emit success toast
```

---

## 8. ID Linking & Localization Architecture (`lang_id` & `update_language_ui`)

In strict compliance with user localization rules:

### 8.1 Principles
1. **No Static Text Replacement**: Never mutate language strings on raw config dictionaries statically at runtime.
2. **ID Linking Property**: Every UI widget that displays human-readable text must be bound to a unique string identifier via `widget.setProperty("lang_id", "string_id_key")`.
3. **Category Property**: Optional property `widget.setProperty("lang_type", "ui")` (or `"server"`, `"modal"`, `"error"`).
4. **Recursive Scanning (`update_language_ui`)**: The UI runtime must execute a recursive tree traversal function `update_language_ui(root_widget)` whenever the active language changes.

### 8.2 ID Linking Data Contract

```rust
/// Structure representing a localized text node binding
pub struct LocalizedWidgetSpec {
    pub lang_id: &'static str,
    pub lang_type: &'static str,
    pub default_text_vi: &'static str,
    pub default_text_en: &'static str,
}

// Example translation dictionary lookup
// "btn_login"          => VI: "Đăng nhập",         EN: "Log In"
// "pane_mode_local"    => VI: "Cục bộ",            EN: "Local"
// "pane_mode_cloud"    => VI: "Cloud",            EN: "Cloud"
// "hdr_file_name"      => VI: "Tên",               EN: "Name"
// "hdr_file_size"      => VI: "Kích thước",        EN: "Size"
// "hdr_file_type"      => VI: "Loại",              EN: "Type"
// "hdr_file_date"      => VI: "Ngày sửa",          EN: "Date Modified"
// "status_ready"       => VI: "Sẵn sàng",          EN: "Ready"
// "modal_mkdir_title"  => VI: "Tạo thư mục mới",   EN: "Create New Directory"
```

### 8.3 Algorithmic Traversal Spec (`update_language_ui`)

```python
def update_language_ui(widget, current_lang_dict):
    """
    Recursively scans widget tree and updates text based on lang_id property.
    """
    if widget.hasProperty("lang_id"):
        lang_id = widget.property("lang_id")
        if lang_id in current_lang_dict:
            new_text = current_lang_dict[lang_id]
            widget.setText(new_text)
    
    # Recursively update child widgets
    for child in widget.children():
        update_language_ui(child, current_lang_dict)
```

---

## 9. Data Schema & UI State Specification

```rust
// UI View State
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum MainView {
    Explorer,
    Recents,
    Sync,
    Servers,
}

// Pane Navigation State
pub struct PaneState {
    pub mode: PaneMode,
    pub path: String,
    pub items: Vec<FileItem>,
    pub filter: String,
    pub selected: Vec<String>,
    pub anchor: Option<String>,
    pub status: String,
    pub back: Vec<String>,
    pub fwd: Vec<String>,
    pub col_w: [f32; 3], // Widths for Size, Type, Date columns
}

// Neon Theme Configuration Spec
pub struct NeonThemeSpec {
    pub bg_obsidian: Color32,
    pub surface_panel: Color32,
    pub surface_glass: Color32,
    pub border_muted: Color32,
    pub neon_cyan: Color32,
    pub neon_magenta: Color32,
    pub neon_purple: Color32,
    pub neon_emerald: Color32,
    pub neon_amber: Color32,
    pub neon_crimson: Color32,
    pub glow_radius_sm: f32,
    pub glow_radius_lg: f32,
}
```

---

## 10. Document Structure Mapping for `docs/` (Requirement R3)

When generating the final UI Design Document inside `apps_gui/filen_gui/docs/`:

```
apps_gui/filen_gui/
├── Cargo.toml
├── src/
│   ├── main.rs
│   ├── operations.rs
│   └── transfer.rs
└── docs/                             <-- REQUIRED BY R3 (sibling to src/)
    ├── NEON_UI_DESIGN_SPEC.md        <-- Comprehensive UI Design Document
    └── USER_FLOW_DIAGRAMS.md         <-- Screen branch & user flow diagrams
```

---
*End of Analysis Report.*
