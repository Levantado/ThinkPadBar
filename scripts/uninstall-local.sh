#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/uninstall-local.sh [--bin-dir DIR] [--dry-run]

Remove a locally installed ThinkPadBar binary from a bin directory.

Options:
  --bin-dir DIR   Source directory of the installed binary (default: ~/.local/bin)
  --dry-run       Print the removal command without executing it
  -h, --help      Show this help
USAGE
}

bin_dir="${HOME}/.local/bin"
dry_run=0

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

while [[ $# -gt 0 ]]; do
  case "$1" in
    --bin-dir)
      bin_dir="${2:-}"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
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

if [[ -z "$bin_dir" ]]; then
  echo "--bin-dir must not be empty" >&2
  exit 1
fi

target="$bin_dir/thinkpadbar"
run rm -f "$target"

if [[ "$dry_run" -eq 1 ]]; then
  echo "Remove 'exec-once = thinkpadbar' from ~/.config/hypr/hyprland.conf if it was added for local autostart."
else
  echo "Removed $target"
  echo "Remove 'exec-once = thinkpadbar' from ~/.config/hypr/hyprland.conf if it was added for local autostart."
fi
