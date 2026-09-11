# Universal Converter — final AppImage smoke evidence
- Status: **PASS** (2026-09-10); this verdict supersedes all older AppImage smoke artifacts/evidence.
- Final workspace artifact: `/home/bimatkeo/Documents/SH/RS_AI/target/release/bundle/appimage/Universal Converter_0.1.0_amd64.AppImage`; 79,890,936 B; SHA-256 `5c2eba9f52e774d1fc045b3782466cb2de4f6d9f7bd1601278089de9e4368045` (matches `package.md`).
- Preserved external CWD/temp: `/tmp/opencode/universal-converter-final-5c2eba9f-20260910`; direct AppImage startup stayed alive to timeout 124; extracted `squashfs-root/AppRun` was then used because host portal/GVFS FUSE mounts were denied.
- Extracted startup PASS: `DISPLAY=:0 XDG_DATA_HOME=... timeout 45s dbus-run-session -- strace -f -e trace=%file .../squashfs-root/AppRun` exited 124, alive without app panic/resource error.
- Exact resources PASS: `langs/{en,vi}.json` (2), `themes/{system,light,dark}.json` (3), exactly `fonts/{DejaVuSans.ttf,LICENSE.txt,manifest.sha256}` (3); all font entries regular physical files, no symlinks.
- Hash PASS: manifest check reports `DejaVuSans.ttf: OK` (`ae7b7855...3837280`) and `LICENSE.txt: OK` (`63d3ba75...f045fbc9`).
- Loader PASS: isolated persisted `font_id=dejavusans` was read; trace records 2 successful `O_RDONLY` opens of bundled `$RESOURCE/fonts/DejaVuSans.ttf`.
- Isolation PASS: 0 source-tree, launch-CWD resource, or cross-GUI fallback matches; system `/usr/share/fonts` reads occurred only as allowed host-stack/system-sentinel behavior.
- Logs/evidence: `artifact.sha256`, `manifest-check.log`, `font-file-types.log`, `final-{exit,stdout,stderr,strace}.log` in the preserved temp above; no temp/artifact deleted.
