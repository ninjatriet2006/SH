# Project: Filen GUI Neon UI Design Document

## Architecture
- Target directory: `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/docs/`
- Output document: `neon_ui_design.md`
- Scope: Complete Neon UI Design System & Technical Specification for filen_GUI.

## Feature Inventory
| # | Feature | Description | Requirement | Milestone | Source |
|---|---------|-------------|-------------|-----------|--------|
| 1 | Neon Color Tokens & Theme | Hex codes (`#080A10`, `#00F3FF`, `#FF007F`, `#9D00FF`, `#00FF87`), glow effects, shadows, dark glassmorphism | R1 | M1 | survey |
| 2 | Typography & Layout Grid | Font scale, weights, spacing grid, container borders | R1 | M1 | survey |
| 3 | Component State Matrix | Hover, Active, Focus, Disabled, Selection glowing states | R1 | M1 | survey |
| 4 | Hybrid Workflow Synthesis | TUI vs GUI workflow comparison & R2 UX superiority integration | R2 | M1 | survey |
| 5 | Screen 1: Auth & 2FA Modal | Login form, 2FA challenge, account selector, state transitions | R1, R2 | M1 | survey |
| 6 | Screen 2: Dual-Pane Explorer | Local/Cloud panes, sidebar, breadcrumb chips, column sort, drag-and-drop highlight, multi-select | R1, R2 | M1 | survey |
| 7 | Screen 3: Recents View | Recents file list, action quick-bar, status indicators | R1, R2 | M1 | survey |
| 8 | Screen 4: Sync Pairs View | Active sync pairs, sync strategy, runner status, log drawer | R1, R2 | M1 | survey |
| 9 | Screen 5: Servers Control | WebDAV, S3, FUSE Mount server launcher cards, port/status controls | R1, R2 | M1 | survey |
| 10| Screen 6: Operation Modals | Mkdir, Rename, Delete confirm, Preview modal, Link generator | R1, R2 | M1 | survey |
| 11| Screen 7: Transfer Queue | Background transfer panel, progress bar neon glow, queue stats | R1, R2 | M1 | survey |
| 12| Screen Branching Flowchart | Complete Mermaid state machine diagram for screen transitions | R3 | M1 | survey |
| 13| UI State Data Model | Rust data structures (`struct`, `enum`) for egui application state | R3 | M1 | survey |
| 14| ID Linking & Localization | `lang_id` system and `update_language_ui` specification | User Rule | M1 | survey |
| 15| Verification & Review | Detailed inspection by Reviewers & Forensic Auditor | Acceptance Criteria | M2 | survey |

## Milestones
| # | Name | Scope | Dependencies | Status |
|---|------|-------|-------------|--------|
| 1 | Neon UI Design Document | Write `docs/neon_ui_design.md` with complete design specs | Survey | DONE |
| 2 | Review & Verification | Independent Reviewer pass + Forensic Auditor integrity check | M1 | DONE |

## Interface Contracts
- **Document Path**: `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/docs/neon_ui_design.md`
- **Sibling Relationship**: `docs/` sits directly alongside `src/` inside `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/`.

## Code Layout
- `apps_gui/filen_gui/docs/neon_ui_design.md` (Design Specification Document)
