#!/usr/bin/env bash
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
FIXTURE="$HERE/tests/fixtures/gui-standalone-audit"

if output="$(AUDIT_ROOT="$FIXTURE" "$HERE/audit-gui-standalone.sh" 2>&1)"; then
  printf '%s\n' "fixture audit unexpectedly passed" >&2
  exit 1
fi

case "$output" in
  *"crosses app_alpha -> app_beta"*) ;;
  *)
    printf '%s\n' "$output" >&2
    printf '%s\n' "fixture audit did not report the cross-app regression" >&2
    exit 1
    ;;
esac

printf '%s\n' "PASS: isolated GUI audit fixture was rejected"
