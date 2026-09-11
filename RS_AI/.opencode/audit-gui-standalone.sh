#!/usr/bin/env bash
set -euo pipefail

ROOT="${AUDIT_ROOT:-$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)}"
export ROOT

python3 <<'PY'
import os
import re
import sys
from pathlib import Path

root = Path(os.environ["ROOT"]).resolve()
gui = root / "GUI"
ignored = {"node_modules", "target", "dist", ".git", "gen"}
manifest_names = {"Cargo.toml", "package.json", "package-lock.json"}
runtime_suffixes = {
    ".rs", ".ts", ".tsx", ".js", ".jsx", ".html", ".css", ".json",
    ".jsonc", ".toml", ".yaml", ".yml",
}
app_names = sorted(p.name for p in gui.iterdir() if p.is_dir())
app_set = set(app_names)
findings = []
scanned = {"manifests": 0, "runtime_files": 0, "symlinks": 0}

def app_for(path: Path):
    try:
        rel = path.relative_to(gui)
    except ValueError:
        return None
    return rel.parts[0] if rel.parts and rel.parts[0] in app_set else None

def record(path: Path, line, match, reason):
    findings.append((path.relative_to(root).as_posix(), line, match, reason))

for dirpath, dirnames, filenames in os.walk(gui, followlinks=False):
    dirnames[:] = sorted(d for d in dirnames if d not in ignored)
    base = Path(dirpath)
    owner = app_for(base)
    for name in sorted(dirnames + filenames):
        path = base / name
        if path.is_symlink():
            scanned["symlinks"] += 1
            target = (path.parent / os.readlink(path)).resolve(strict=False)
            target_owner = app_for(target)
            if owner and target_owner and owner != target_owner:
                record(path, "symlink", os.readlink(path), f"symlink crosses {owner} -> {target_owner}")

    for name in sorted(filenames):
        path = base / name
        if path.is_symlink() or not path.is_file():
            continue
        is_manifest = name in manifest_names
        is_runtime_file = path.suffix in runtime_suffixes
        if not (is_manifest or is_runtime_file):
            continue
        scanned["manifests" if is_manifest else "runtime_files"] += 1
        try:
            text = path.read_text(encoding="utf-8")
        except UnicodeDecodeError:
            continue
        owner = app_for(path)
        if not owner:
            continue

        # Explicit references to another GUI app are forbidden in manifests and runtime sources.
        for other in app_names:
            if other == owner:
                continue
            pattern = re.compile(rf"(?:GUI[/\\])?{re.escape(other)}(?:[/\\]|\b)")
            for number, line in enumerate(text.splitlines(), 1):
                if pattern.search(line):
                    record(path, number, line.strip(), f"explicit reference crosses {owner} -> {other}")

        # Resolve relative TS/JS imports; a resolved destination in another GUI is forbidden.
        if path.suffix in {".ts", ".tsx", ".js", ".jsx"}:
            imports = re.finditer(
                r"(?:from\s*|import\s*\(|require\s*\()\s*['\"](\.{1,2}/[^'\"]+)['\"]",
                text,
            )
            for match in imports:
                spec = match.group(1)
                target = (path.parent / spec).resolve(strict=False)
                target_owner = app_for(target)
                if target_owner and target_owner != owner:
                    line = text.count("\n", 0, match.start()) + 1
                    record(path, line, spec, f"relative import crosses {owner} -> {target_owner}")

        # Resolve quoted relative runtime/config paths (Rust include/resource paths,
        # CSS/HTML assets, Tauri config, and similar). This catches generic paths
        # that cross an app boundary without spelling another app's name.
        runtime_paths = re.finditer(r"['\"]((?:\.{1,2}[/\\])+[^'\"\r\n]+)['\"]", text)
        for match in runtime_paths:
            spec = match.group(1)
            target = (path.parent / spec).resolve(strict=False)
            target_owner = app_for(target)
            if target_owner and target_owner != owner:
                line = text.count("\n", 0, match.start()) + 1
                record(path, line, spec, f"runtime/config path crosses {owner} -> {target_owner}")

        # Resolve local path-like manifest values, including Cargo path dependencies.
        if is_manifest:
            values = re.finditer(r"['\"]((?:\.{1,2}/)+[^'\"]+)['\"]", text)
            for match in values:
                spec = match.group(1)
                target = (path.parent / spec).resolve(strict=False)
                target_owner = app_for(target)
                if target_owner and target_owner != owner:
                    line = text.count("\n", 0, match.start()) + 1
                    record(path, line, spec, f"manifest path crosses {owner} -> {target_owner}")

print("# GUI standalone audit")
print()
print(f"- Root: `{root}`")
print("- Scope: `GUI/` Cargo/npm manifests, Rust/web runtime source, HTML/CSS/JSON/TOML/YAML config, and symlinks")
print(f"- Excluded generated trees: `{', '.join(sorted(ignored))}`")
print(f"- Scanned: {scanned['manifests']} manifests, {scanned['runtime_files']} runtime/config files, {scanned['symlinks']} symlinks")
print("- Allowlist: none")
print()
if findings:
    print("## FAIL — non-allowlisted GUI-to-GUI references")
    print()
    for path, line, match, reason in sorted(set(findings)):
        print(f"- `{path}:{line}` — {reason}; match: `{match}`")
    sys.exit(1)
print("## PASS — no non-allowlisted GUI-to-GUI reference found")
PY
