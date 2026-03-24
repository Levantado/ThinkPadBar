# Runtime Metrics

This project includes `scripts/runtime-metrics.sh` to collect basic runtime metrics
for `thinkpadbar` (or any process name/PID) on Linux.

## What It Collects
- CPU usage (`%CPU`) samples
- RSS memory (`rss_kb`) samples
- Threads and FD count over time
- Context switch deltas (`voluntary` / `nonvoluntary`)
- CSV output for later comparison

## Quick Start
```bash
# Default: process name thinkpadbar, 60s, 2s interval
./scripts/runtime-metrics.sh

# Explicit PID and custom window
./scripts/runtime-metrics.sh --pid 1576 --duration 120 --interval 2

# Save CSV to specific path
./scripts/runtime-metrics.sh --output /tmp/thinkpadbar-idle.csv
```

## Suggested Profiles
```bash
# Idle baseline
./scripts/runtime-metrics.sh --duration 120 --interval 2 --output /tmp/thinkpadbar-idle.csv

# During build load (run in parallel with cargo check/build)
./scripts/runtime-metrics.sh --duration 120 --interval 2 --output /tmp/thinkpadbar-load.csv
```

## Interpreting Results
- Focus on `CPU avg/min/max` and `RSS avg/min/max` deltas between idle/load runs.
- Rising RSS across repeated idle runs indicates potential retention/leak behavior.
- High nonvoluntary context-switch delta at idle may indicate excessive wakeups/contention.
