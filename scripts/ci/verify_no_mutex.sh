#!/usr/bin/env bash
#
# verify_no_mutex.sh — M-001 sentinel (R-002 in docs/archived/perf/perfBusFastPath/03_REVIEW.md).
#
# Rejects any attempt to wrap `Bus` in `Mutex`, `RwLock`, `parking_lot::{Mutex,RwLock}`,
# or `Arc<Mutex<Bus>>` anywhere under xemu/xcore/src/. The check is a *type-shape*
# regex over the whole tree (not a fixed file allow-list), so new files added on
# the bus path cannot silently re-introduce the forbidden shape.
#
# Exit codes:
#   0 — clean (no matches)
#   1 — violation (one or more matches printed to stderr)
#
# Invoked from `make test` and from the migration commit's pre-merge gate.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
SRC="$ROOT/xemu/xcore/src"

if [[ ! -d "$SRC" ]]; then
  echo "verify_no_mutex: source tree not found at $SRC" >&2
  exit 2
fi

# Type-shape regex — catches:
#   Mutex<Bus>, Mutex<  Bus  >
#   RwLock<Bus>, parking_lot::Mutex<Bus>, parking_lot::RwLock<Bus>
#   Arc<Mutex<Bus>, Arc<RwLock<Bus>
PATTERNS=(
  'Mutex<\s*Bus\s*>'
  'RwLock<\s*Bus\s*>'
  'parking_lot::(Mutex|RwLock)<\s*Bus'
  'Arc<\s*Mutex<\s*Bus'
  'Arc<\s*RwLock<\s*Bus'
  'Arc<\s*parking_lot::(Mutex|RwLock)<\s*Bus'
)

violations=0
for pattern in "${PATTERNS[@]}"; do
  # Strip comment/docstring lines (starting with `//`, `///`, `//!`, or inside a
  # `compile_fail` block introduced by `//!`) before matching the type-shape regex.
  # Implementation: rg prints matches; pipe through a grep filter that keeps only
  # lines NOT prefixed by `//` (with optional whitespace).
  if rg -U --pcre2 -n "$pattern" "$SRC" 2>/dev/null \
     | grep -vE ':[[:space:]]*//' >/dev/null; then
    rg -U --pcre2 -n "$pattern" "$SRC" 2>/dev/null \
       | grep -vE ':[[:space:]]*//' >&2 || true
    violations=$((violations + 1))
  fi
done

if [[ $violations -gt 0 ]]; then
  echo "verify_no_mutex: M-001 violation — Bus must not be wrapped in a synchronization primitive." >&2
  echo "verify_no_mutex: see docs/archived/perf/perfBusFastPath/01_MASTER.md" >&2
  exit 1
fi

echo "verify_no_mutex: ok"
