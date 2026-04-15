# shellcheck shell=bash
# scripts/perf/lib/_common.sh — tiny shared helpers for perf scripts.

set -uo pipefail
# NOTE: no `set -e` — the inner `make run` chain sometimes exits with
# non-zero status even when the benchmark completed fine (e.g. because
# of the `HIT GOOD TRAP` handling propagating through xdb's exit code).
# We check return codes explicitly where correctness matters.

log()  { printf '\033[1;34m[perf]\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33m[warn]\033[0m %s\n' "$*" >&2; }
die()  { printf '\033[1;31m[err]\033[0m  %s\n' "$*" >&2; exit 1; }

# Parse `--out=DIR` / `--out DIR`, filling OUT_DIR.  Remaining positional
# args are returned in the POSITIONAL array (bash arrays don't survive
# `set --` across function boundaries).
parse_out() {
  OUT_DIR="${OUT_DIR:-$DEFAULT_OUT}"
  POSITIONAL=()
  while [ $# -gt 0 ]; do
    case $1 in
      --out=*) OUT_DIR="${1#--out=}"; shift ;;
      --out)   OUT_DIR="$2"; shift 2 ;;
      -h|--help) usage; exit 0 ;;
      *) POSITIONAL+=("$1"); shift ;;
    esac
  done
  mkdir -p "$OUT_DIR/data" "$OUT_DIR/graphics"
}

# Echo "real user sys rss_kb" from a macOS `time -l` file.
parse_time_l() {
  awk '
    / real / { real=$1; user=$3; sys=$5 }
    /maximum resident set size/ { rss=$1 }
    END { printf "%s %s %s %d", real, user, sys, rss/1024 }
  ' "$1"
}
