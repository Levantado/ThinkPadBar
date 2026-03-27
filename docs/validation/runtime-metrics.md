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

## Perf Smoke

For a quick pass/fail check instead of manual CSV inspection, use:

```bash
./scripts/perf-smoke.sh
```

This script can:
- launch `target/release/thinkpadbar` automatically, or use `--installed`;
- measure an existing PID with `--pid`;
- validate simple thresholds for max RSS, average CPU, and max thread count;
- load a baseline/threshold profile from a key=value file with `--profile-file`;
- write the current run back as a baseline profile with `--write-profile`;
- reuse `scripts/runtime-metrics.sh` for CSV generation.

Examples:

```bash
# Default smoke against target/release/thinkpadbar
./scripts/perf-smoke.sh

# Measure the installed binary from PATH
./scripts/perf-smoke.sh --installed

# Use stricter limits for a regression check
./scripts/perf-smoke.sh --duration 60 --max-rss-kb 70000 --max-cpu-avg 4 --max-threads 32

# Compare against a saved baseline profile
./scripts/perf-smoke.sh --profile-file docs/validation/perf-smoke.profile.example

# Save the current run as a new baseline profile
./scripts/perf-smoke.sh --write-profile /tmp/thinkpadbar.profile

# Measure an already-running ThinkPadBar instance
./scripts/perf-smoke.sh --pid 1576 --duration 60 --output /tmp/thinkpadbar-smoke.csv

# Preview the exact commands
./scripts/perf-smoke.sh --dry-run
```

An example profile is included at `docs/validation/perf-smoke.profile.example`.

## Interpreting Results
- Focus on `CPU avg/min/max` and `RSS avg/min/max` deltas between idle/load runs.
- Rising RSS across repeated idle runs indicates potential retention/leak behavior.
- High nonvoluntary context-switch delta at idle may indicate excessive wakeups/contention.
