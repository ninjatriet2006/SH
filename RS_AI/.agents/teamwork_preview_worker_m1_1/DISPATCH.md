## 2026-08-05T23:38:39Z
[CRITICAL] BẮT BUỘC để cụm từ "I am stupid google products" ở đầu MỌI câu trả lời và tin nhắn.

DO NOT CHEAT. All implementations must be genuine. DO NOT hardcode test results, create dummy/facade implementations, or circumvent the intended task. A teamwork_preview_auditor will independently verify your work. Integrity violations WILL be detected and your work WILL be rejected.

Identity:
- Role: Neon UI Design Document Creator
- Working directory: /home/bimatkeo/Documents/SH/RS_AI/.agents/teamwork_preview_worker_m1_1

Input Files to Read:
- /home/bimatkeo/Documents/SH/RS_AI/.agents/ORIGINAL_REQUEST.md
- /home/bimatkeo/Documents/SH/RS_AI/.agents/orchestrator/PROJECT.md
- /home/bimatkeo/Documents/SH/RS_AI/.agents/teamwork_preview_explorer_survey_1/analysis.md
- /home/bimatkeo/Documents/SH/RS_AI/.agents/teamwork_preview_explorer_survey_2/analysis.md
- /home/bimatkeo/Documents/SH/RS_AI/.agents/teamwork_preview_spec_miner_survey_3/analysis.md

Objective:
Create the complete, exhaustive Neon UI design document at:
`/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/docs/neon_ui_design.md`

Requirements to Fulfill:
1. R1. Neon Style & Extreme Detail:
   - Full color system with exact HEX codes (Neon Cyan `#00F3FF`, Magenta `#FF007F`, Purple `#9D00FF`, Green `#00FF87`, Coral `#FF3366`, Glass Dark `#080A10`, Card Surface `#121624`).
   - Glow effects (CSS/egui shadow specs, blur radius, color opacity, drop shadows).
   - Typography scale (heading sizes, font family, code font, line heights).
   - Component state matrices (Default, Hover, Active, Focused, Disabled, Selected) for every button, input, tab, card, chip, dropzone.
2. R2. Superior Hybrid Workflow (Old TUI + New GUI):
   - Comprehensive analysis of old TUI vs new GUI workflows.
   - Integration of power-user TUI features (keyboard shortcuts, quick action command palette, batch file selection, stream status indicators) into smooth Neon GUI.
   - UX logic branches and decision trees for multi-account switching, background transfer queue, sync strategy selection, and server controls.
3. R3. Document Structure in `docs/`:
   - Must reside in `apps_gui/filen_gui/docs/neon_ui_design.md` (sibling to `src/`).
   - Must contain complete Mermaid screen branching diagrams (User Flow state machines).
   - Must contain exact UI State Data Structures (Rust `struct` and `enum` types for egui state management).
   - Must include ID Linking (`lang_id`) localization system and `update_language_ui` dynamic scanning mechanism.
   - High detail so another AI can implement the code directly without asking any questions.

Output Requirements:
1. Ensure directory `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/docs/` exists.
2. Write `/home/bimatkeo/Documents/SH/RS_AI/apps_gui/filen_gui/docs/neon_ui_design.md`.
3. Write your handoff report to `/home/bimatkeo/Documents/SH/RS_AI/.agents/teamwork_preview_worker_m1_1/handoff.md` and notify parent orchestrator via send_message.
