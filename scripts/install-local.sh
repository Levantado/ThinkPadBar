#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'USAGE'
Usage: scripts/install-local.sh [--bin-dir DIR] [--debug] [--dry-run]

Build ThinkPadBar with the repository lockfile and install the binary locally.

Options:
  --bin-dir DIR   Target directory for the installed binary (default: ~/.local/bin)
  --debug         Build the debug profile instead of release
  --dry-run       Print the commands without executing them
  -h, --help      Show this help
USAGE
}

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
bin_dir="${HOME}/.local/bin"
profile="release"
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
    --debug)
      profile="debug"
      shift
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

build_cmd=(cargo build --locked)
binary_path="$repo_root/target/debug/thinkpadbar"

if [[ "$profile" == "release" ]]; then
  build_cmd+=(--release)
  binary_path="$repo_root/target/release/thinkpadbar"
fi

install_cmd=(install -Dm755 "$binary_path" "$bin_dir/thinkpadbar")

if [[ "$dry_run" -eq 1 ]]; then
  echo "# repo: $repo_root"
fi

(
  cd "$repo_root"
  run "${build_cmd[@]}"
)
run "${install_cmd[@]}"

if [[ "$dry_run" -eq 1 ]]; then
  echo "Add 'exec-once = thinkpadbar' to ~/.config/hypr/hyprland.conf if not already present."
else
  echo "Installed thinkpadbar to $bin_dir/thinkpadbar"
  echo "Add 'exec-once = thinkpadbar' to ~/.config/hypr/hyprland.conf if not already present."
fi
