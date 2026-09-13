# Universe Manager GUI — Phase 7.7 external-CWD smoke evidence

- Status: **PASS** (fresh rerun 2026-09-11 after five Reviewer fixes).
- External CWD `/tmp/opencode/universe-reviewer-fixes-95e467b-20260911-qa/cwd-decoys`: extracted DEB, extracted AppRun and direct `APPIMAGE_EXTRACT_AND_RUN=1` launches each stayed alive to the 10-second timeout (exit 124), with empty logs.
- Restart persistence PASS in isolated XDG roots: canonical `preferences.json` retained `en/dark/dejavusans`; extracted AppRun stayed alive after restart (exit 124), with an empty log.
- Fallback matrix PASS 8/8: missing/corrupt EN, missing/corrupt VI, missing/corrupt system theme, and missing/corrupt DejaVu font each stayed alive to the 8-second timeout (exit 124), with empty logs.
- CWD contained decoy `langs`, `themes`, `fonts`, and `GUI` trees; bridge static/runtime tests reject CWD and cross-bundle symlinks. No source/CWD/cross-GUI resource lookup was observed.
- Package metadata, all bundled resources and hashes, font license/manifest, and updater-off contract PASS. MSI/DMG remain `deferred/platform-unverified`.
- Phase 8.6 refresh (2026-09-12): AppImage `eda1b862...28aa` launched from `/tmp`; alive to timeout 124 with an empty log.
- All extracted artifacts, isolated XDG roots, original resources renamed beside mutated fixtures, and logs are preserved under the smoke root; nothing deleted.
- Post-four-fix Phase 8.6: fresh AppImage `dfd954e0...53cac0` and extracted DEB binary passed isolated external-CWD/XDG launch (timeout 124); extraction/resources/font manifest PASS. Evidence root: `/tmp/opencode/phase-8-four-fixes-20260912T045604Z-272465`.
- Reviewer F1 refresh (2026-09-13): fresh AppImage and extracted DEB binary both passed isolated external-CWD/XDG smoke (timeout 124); SHA verification, DEB metadata, regular resources, and font manifest PASS. Durable root: `/tmp/opencode/phase8-f1-five-20260913T000000Z/universe`.
