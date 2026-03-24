#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<USAGE
Usage: $0 [--pid PID] [--name PROC_NAME] [--duration SEC] [--interval SEC] [--output FILE]

Collect runtime metrics (CPU/RSS/threads/fd/context-switches) for a running process.

Options:
  --pid PID         Explicit process PID (default: auto-discover by --name)
  --name NAME       Process name for auto-discovery via pgrep -x (default: thinkpadbar)
  --duration SEC    Sampling duration in seconds (default: 60)
  --interval SEC    Sampling interval in seconds (default: 2)
  --output FILE     CSV output path (default: /tmp/<name>-metrics-<timestamp>.csv)
  -h, --help        Show this help
USAGE
}

PID=""
NAME="thinkpadbar"
DURATION=60
INTERVAL=2
OUTPUT=""

while [[ $# -gt 0 ]]; do
  case "$1" in
    --pid)
      PID="${2:-}"
      shift 2
      ;;
    --name)
      NAME="${2:-}"
      shift 2
      ;;
    --duration)
      DURATION="${2:-}"
      shift 2
      ;;
    --interval)
      INTERVAL="${2:-}"
      shift 2
      ;;
    --output)
      OUTPUT="${2:-}"
      shift 2
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

if [[ -z "$PID" ]]; then
  PID="$(pgrep -x "$NAME" | head -n1 || true)"
fi

if [[ -z "$PID" || ! -d "/proc/$PID" ]]; then
  echo "Process not found (name=$NAME pid=$PID)" >&2
  exit 1
fi

if ! [[ "$DURATION" =~ ^[0-9]+$ ]] || ! [[ "$INTERVAL" =~ ^[0-9]+$ ]] || [[ "$INTERVAL" -le 0 ]]; then
  echo "duration/interval must be positive integers" >&2
  exit 1
fi

if [[ -z "$OUTPUT" ]]; then
  TS="$(date +%Y%m%d-%H%M%S)"
  OUTPUT="/tmp/${NAME}-metrics-${TS}.csv"
fi

SAMPLES=$(( DURATION / INTERVAL ))
if [[ "$SAMPLES" -le 0 ]]; then
  SAMPLES=1
fi

read_ctx_switches() {
  local pid="$1"
  awk '
    /voluntary_ctxt_switches/ {v=$2}
    /nonvoluntary_ctxt_switches/ {n=$2}
    END {print v+0, n+0}
  ' "/proc/$pid/status"
}

read_threads() {
  local pid="$1"
  awk '/Threads/ {print $2+0}' "/proc/$pid/status"
}

read_fd_count() {
  local pid="$1"
  ls "/proc/$pid/fd" 2>/dev/null | wc -l | awk '{print $1+0}'
}

read -r ctx_v0 ctx_nv0 < <(read_ctx_switches "$PID")
threads0="$(read_threads "$PID")"
fd0="$(read_fd_count "$PID")"

{
  echo "timestamp,cpu_percent,rss_kb,threads,fd_count"
} > "$OUTPUT"

cpu_sum=0
rss_sum=0
cpu_min=1000000
cpu_max=-1
rss_min=1000000000
rss_max=-1
count=0

for _ in $(seq 1 "$SAMPLES"); do
  if [[ ! -d "/proc/$PID" ]]; then
    echo "Process exited during sampling" >&2
    break
  fi

  ts="$(date +%H:%M:%S)"
  read -r cpu rss < <(ps -p "$PID" -o %cpu=,rss= | awk '{print $1+0, $2+0}')
  th="$(read_threads "$PID")"
  fd="$(read_fd_count "$PID")"

  echo "${ts},${cpu},${rss},${th},${fd}" >> "$OUTPUT"

  cpu_sum=$(awk -v a="$cpu_sum" -v b="$cpu" 'BEGIN{printf "%.6f", a+b}')
  rss_sum=$((rss_sum + rss))

  cpu_min=$(awk -v a="$cpu_min" -v b="$cpu" 'BEGIN{print (b<a)?b:a}')
  cpu_max=$(awk -v a="$cpu_max" -v b="$cpu" 'BEGIN{print (b>a)?b:a}')
  (( rss < rss_min )) && rss_min=$rss
  (( rss > rss_max )) && rss_max=$rss
  count=$((count + 1))

  sleep "$INTERVAL"
done

if [[ "$count" -eq 0 ]]; then
  echo "No samples collected" >&2
  exit 1
fi

read -r ctx_v1 ctx_nv1 < <(read_ctx_switches "$PID" || echo "0 0")
threads1="$(read_threads "$PID" || echo 0)"
fd1="$(read_fd_count "$PID" || echo 0)"

cpu_avg=$(awk -v s="$cpu_sum" -v c="$count" 'BEGIN{printf "%.3f", s/c}')
rss_avg=$((rss_sum / count))

cat <<SUMMARY
=== Runtime Metrics Summary ===
Process: $NAME (pid=$PID)
Samples: $count (duration=${DURATION}s, interval=${INTERVAL}s)
CPU%   avg/min/max: ${cpu_avg} / ${cpu_min} / ${cpu_max}
RSS KB avg/min/max: ${rss_avg} / ${rss_min} / ${rss_max}
Threads start/end: ${threads0} -> ${threads1}
FDs     start/end: ${fd0} -> ${fd1}
Context switches (delta):
  voluntary: $((ctx_v1 - ctx_v0))
  nonvoluntary: $((ctx_nv1 - ctx_nv0))
CSV: $OUTPUT
SUMMARY
