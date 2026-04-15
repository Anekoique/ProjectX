#!/usr/bin/env bash
# scripts/perf/sample.sh — CPU sampling profile of xemu via Apple `sample`.
#
# For each requested workload, launches `make run` in the background,
# attaches /usr/bin/sample to the child `xdb` PID for the workload's
# sample-seconds window, and saves a call-tree text profile at
# <OUT>/data/<workload>.sample.txt.
#
# Usage:
#   scripts/perf/sample.sh [workload ...] [--out DIR]
# With no workload names, iterates over $WORKLOADS.
#
# Requires macOS (`/usr/bin/sample`).  On Linux, use `samply record`
# against `make run` instead (no codesign required on Linux).

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=config.sh
source "$HERE/config.sh"
# shellcheck source=lib/_common.sh
source "$HERE/lib/_common.sh"

usage() { sed -n '2,14p' "$0" | sed 's/^# \{0,1\}//'; }

parse_out "$@"
requested=("${POSITIONAL[@]+"${POSITIONAL[@]}"}")

# Return the descendants of a given PID, one per line, transitively.
descendants() {
  local root=$1
  local frontier="$root"
  local all=""
  while [ -n "$frontier" ]; do
    local next=""
    for p in $frontier; do
      local kids
      kids=$(pgrep -P "$p" 2>/dev/null || true)
      [ -n "$kids" ] && next="$next $kids"
    done
    all="$all $next"
    frontier="$next"
  done
  # Strip leading spaces, one PID per line.
  echo "$all" | tr ' ' '\n' | sed '/^$/d'
}

# Find exactly-one xdb PID that is a descendant of $make_pid; fail
# deterministically on zero or >1 matches.
find_xdb_descendant() {
  local make_pid=$1 deadline=$2
  local now
  while [ "$(date +%s)" -lt "$deadline" ]; do
    local candidates
    candidates=$(descendants "$make_pid" | while read -r p; do
      [ -z "$p" ] && continue
      if ps -p "$p" -o comm= 2>/dev/null | grep -Eq '(/|^)xdb$'; then
        echo "$p"
      fi
    done)
    local n
    n=$(printf '%s\n' "$candidates" | sed '/^$/d' | wc -l | tr -d ' ')
    if [ "$n" = "1" ]; then
      echo "$candidates"
      return 0
    fi
    if [ "$n" -gt 1 ]; then
      warn "multiple xdb descendants of make pid $make_pid: $candidates"
      return 2
    fi
    sleep 0.1
  done
  return 1
}

sample_one() {
  local name=$1 dir=$2 dur=$3
  local out="$OUT_DIR/data/${name}.sample.txt"
  # Truncate any prior profile so we never mistake it for fresh output.
  : >"$out"
  log "sample $name  window=${dur}s  out=$out"
  ( cd "$dir" && make kernel MODE="$PERF_MODE" >/dev/null 2>&1 ) \
    || warn "prebuild of $name failed, continuing"
  ( cd "$dir" && make run MODE="$PERF_MODE" ) >/dev/null 2>&1 &
  local make_pid=$!
  local deadline=$(( $(date +%s) + 8 ))
  local xdb
  if ! xdb=$(find_xdb_descendant "$make_pid" "$deadline"); then
    kill -TERM "$make_pid" 2>/dev/null || true
    wait "$make_pid" 2>/dev/null || true
    die "$name: could not resolve a single xdb descendant of make pid $make_pid"
  fi
  log "  attaching /usr/bin/sample to pid $xdb"
  if ! /usr/bin/sample "$xdb" "$dur" 1 -wait -mayDie -file "$out" >/dev/null; then
    warn "$name: /usr/bin/sample exited non-zero (partial profile at $out)"
  fi
  wait "$make_pid" 2>/dev/null || true
  if [ ! -s "$out" ]; then
    die "$name: empty profile at $out (sample failed)"
  fi
}

for entry in "${WORKLOADS[@]}"; do
  read -r name dir dur <<<"$entry"
  if [ ${#requested[@]} -eq 0 ] || [[ " ${requested[*]} " == *" $name "* ]]; then
    sample_one "$name" "$dir" "$dur"
  fi
done

log "done"
