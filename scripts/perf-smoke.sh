#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/perf-smoke.sh [options] [-- command...]

Run a small runtime performance smoke check for ThinkPadBar (or another command)
using scripts/runtime-metrics.sh and fail if simple thresholds are exceeded.

Options:
  --pid PID            Measure an already-running process instead of launching one
  --name NAME          Process name for metrics summary/output naming (default: thinkpadbar)
  --duration SEC       Sampling duration in seconds (default: 30)
  --interval SEC       Sampling interval in seconds (default: 2)
  --settle SEC         Wait time after launch before sampling (default: 3)
  --max-rss-kb KB      Fail if max RSS exceeds this value (default: 80000)
  --max-cpu-avg PCT    Fail if average CPU exceeds this value (default: 5)
  --max-threads N      Fail if max thread count exceeds this value (default: 40)
  --output FILE        CSV output path (default: /tmp/<name>-perf-smoke-<timestamp>.csv)
  --installed          Launch thinkpadbar from PATH instead of target/release/thinkpadbar
  --dry-run            Print the commands and thresholds without executing them
  -h, --help           Show this help

Examples:
  ./scripts/perf-smoke.sh
  ./scripts/perf-smoke.sh --installed --duration 60 --max-rss-kb 70000
  ./scripts/perf-smoke.sh --pid 1576 --duration 120 --output /tmp/thinkpadbar.csv
  ./scripts/perf-smoke.sh -- env RUST_LOG=thinkpadbar=debug target/release/thinkpadbar
USAGE
}

quote_cmd() {
  printf '%q ' "$@"
  printf '\n'
}

run() {
  if [[ "$dry_run" -eq 1 ]]; then
    quote_cmd "$@"
  else
    "$@"
  fi
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
metrics_script="$repo_root/scripts/runtime-metrics.sh"

pid=""
name="thinkpadbar"
duration=30
interval=2
settle=3
max_rss_kb=80000
max_cpu_avg=5
max_threads=40
output=""
installed=0
dry_run=0
launch_command=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid)
      pid="${2:-}"
      shift 2
      ;;
    --name)
      name="${2:-}"
      shift 2
      ;;
    --duration)
      duration="${2:-}"
      shift 2
      ;;
    --interval)
      interval="${2:-}"
      shift 2
      ;;
    --settle)
      settle="${2:-}"
      shift 2
      ;;
    --max-rss-kb)
      max_rss_kb="${2:-}"
      shift 2
      ;;
    --max-cpu-avg)
      max_cpu_avg="${2:-}"
      shift 2
      ;;
    --max-threads)
      max_threads="${2:-}"
      shift 2
      ;;
    --output)
      output="${2:-}"
      shift 2
      ;;
    --installed)
      installed=1
      shift
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --)
      shift
      launch_command=("$@")
      break
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "Unknown arg: $1" >&2
      usage >&2
      exit 1
      ;;
  esac
done

for value in "$duration" "$interval" "$settle" "$max_rss_kb" "$max_threads"; do
  if ! [[ "$value" =~ ^[0-9]+$ ]]; then
    echo "duration, interval, settle, max-rss-kb, and max-threads must be positive integers" >&2
    exit 1
  fi
done

if ! [[ "$max_cpu_avg" =~ ^[0-9]+([.][0-9]+)?$ ]]; then
  echo "--max-cpu-avg must be a positive number" >&2
  exit 1
fi

if [[ -z "$output" ]]; then
  ts="$(date +%Y%m%d-%H%M%S)"
  output="/tmp/${name}-perf-smoke-${ts}.csv"
fi

if [[ -z "$pid" && "${#launch_command[@]}" -eq 0 ]]; then
  if [[ "$installed" -eq 1 ]]; then
    launch_command=("thinkpadbar")
  else
    launch_command=("$repo_root/target/release/thinkpadbar")
  fi
fi

launched_pid=""

cleanup() {
  if [[ -n "$launched_pid" ]] && kill -0 "$launched_pid" 2>/dev/null; then
    kill "$launched_pid" 2>/dev/null || true
    wait "$launched_pid" 2>/dev/null || true
  fi
}

trap cleanup EXIT

if [[ "$dry_run" -eq 1 ]]; then
  echo "# repo: $repo_root"
  echo "# thresholds: max_rss_kb=$max_rss_kb max_cpu_avg=$max_cpu_avg max_threads=$max_threads duration=$duration interval=$interval settle=$settle"
fi

if [[ -n "$pid" ]]; then
  if [[ "$dry_run" -eq 1 ]]; then
    echo "# measuring existing pid=$pid"
  elif [[ ! -d "/proc/$pid" ]]; then
    echo "PID $pid is not running" >&2
    exit 1
  fi
else
  if [[ "$dry_run" -eq 1 ]]; then
    run "${launch_command[@]}"
    echo "# sleep $settle"
    run "$metrics_script" --pid 424242 --name "$name" --duration "$duration" --interval "$interval" --output "$output"
    exit 0
  fi

  (
    cd "$repo_root"
    "${launch_command[@]}"
  ) &
  launched_pid="$!"
  pid="$launched_pid"
  sleep "$settle"

  if ! kill -0 "$pid" 2>/dev/null; then
    echo "Launched process exited before sampling started" >&2
    exit 1
  fi
fi

run "$metrics_script" --pid "$pid" --name "$name" --duration "$duration" --interval "$interval" --output "$output"

read -r rss_max threads_max cpu_avg < <(
  awk -F, '
    NR == 1 { next }
    {
      cpu_sum += $2
      if ($3 > rss_max) rss_max = $3
      if ($4 > threads_max) threads_max = $4
      count++
    }
    END {
      if (count == 0) {
        print 0, 0, 0
      } else {
        printf "%d %d %.3f\n", rss_max, threads_max, cpu_sum / count
      }
    }
  ' "$output"
)

echo "=== Perf Smoke Thresholds ==="
echo "RSS max     : ${rss_max} KB (limit ${max_rss_kb} KB)"
echo "CPU avg     : ${cpu_avg}% (limit ${max_cpu_avg}%)"
echo "Threads max : ${threads_max} (limit ${max_threads})"

failures=0

if (( rss_max > max_rss_kb )); then
  echo "FAIL: RSS max exceeded threshold" >&2
  failures=1
fi

if awk -v actual="$cpu_avg" -v limit="$max_cpu_avg" 'BEGIN { exit !(actual > limit) }'; then
  echo "FAIL: CPU avg exceeded threshold" >&2
  failures=1
fi

if (( threads_max > max_threads )); then
  echo "FAIL: thread count exceeded threshold" >&2
  failures=1
fi

if [[ "$failures" -ne 0 ]]; then
  exit 1
fi

echo "PASS: perf smoke thresholds satisfied"
