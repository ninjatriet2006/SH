#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$HERE/tests/fixtures/gui-standalone-audit"

if output="$(AUDIT_ROOT="$FIXTURE" AUDIT_EXPECTED_GUI_ROOTS="app_alpha,app_beta" "$HERE/audit-gui-standalone.sh" 2>&1)"; then
  printf '%s\n' "fixture audit unexpectedly passed" >&2
  exit 1
fi

for expected in \
  "relative import crosses app_alpha -> app_beta" \
  "manifest path crosses app_alpha -> app_beta" \
  "symlink crosses app_alpha -> app_beta" \
  "runtime/config path crosses app_alpha -> app_beta" \
  "allowlist entry permits GUI coupling via app_beta"; do
  case "$output" in
    *"$expected"*) ;;
    *)
    printf '%s\n' "$output" >&2
    printf '%s\n' "fixture audit did not report: $expected" >&2
    exit 1
      ;;
  esac
done

printf '%s\n' "PASS: 5 isolated GUI coupling fixture cases were rejected"
