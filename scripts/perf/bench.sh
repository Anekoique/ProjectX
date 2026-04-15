#!/usr/bin/env bash
# scripts/perf/bench.sh — wall-clock + peak-RSS benchmarks for xemu.
#
# Runs every workload in $WORKLOADS through its own `make run`, N iterations
# each, and writes <OUT>/data/bench.csv plus raw per-run artifacts.
#
# Usage:
#   scripts/perf/bench.sh [--out DIR] [--runs N]
#
# Defaults:
#   --out   docs/perf/<today>
#   --runs  3

HERE="$(cd "$(dirname "$0")" && pwd)"
# shellcheck source=config.sh
source "$HERE/config.sh"
# shellcheck source=lib/_common.sh
source "$HERE/lib/_common.sh"

usage() { sed -n '2,13p' "$0" | sed 's/^# \{0,1\}//'; }

RUNS=3
EXTRA=()
for arg in "$@"; do
  case $arg in
    --runs=*) RUNS="${arg#--runs=}" ;;
    *)        EXTRA+=("$arg") ;;
  esac
done
parse_out "${EXTRA[@]+"${EXTRA[@]}"}"

csv="$OUT_DIR/data/bench.csv"
summary="$OUT_DIR/data/bench.summary"
echo "workload,run,real_s,user_s,sys_s,max_rss_kb" >"$csv"
{
  echo "# perf run  rev=$PERF_REV  mode=$PERF_MODE  date=$(date -u +%Y-%m-%dT%H:%M:%SZ)"
  echo "# host: $(uname -sr) $(uname -m)"
} >"$summary"

run_workload() {
  local name=$1 dir=$2
  local log="$OUT_DIR/data/${name}.log"
  : >"$log"
  log "$name × $RUNS  (make run in $dir)"
  echo ">> $name × $RUNS" >>"$summary"
  ( cd "$dir" && make kernel MODE="$PERF_MODE" >/dev/null 2>&1 ) \
    || warn "prebuild of $name failed, continuing"
  for i in $(seq 1 "$RUNS"); do
    local tfile="$OUT_DIR/data/${name}.run${i}.time"
    ( cd "$dir" && /usr/bin/time -l make run MODE="$PERF_MODE" ) \
      >>"$log" 2>"$tfile"
    read -r real user sys rss < <(parse_time_l "$tfile")
    printf '%s,%d,%s,%s,%s,%d\n' "$name" "$i" "$real" "$user" "$sys" "$rss" >>"$csv"
    printf '  run%d  real=%ss  user=%ss  sys=%ss  rss=%dKiB\n' \
      "$i" "$real" "$user" "$sys" "$rss" | tee -a "$summary"
  done
}

for entry in "${WORKLOADS[@]}"; do
  read -r name dir _ <<<"$entry"
  run_workload "$name" "$dir"
done

log "wrote $csv"
