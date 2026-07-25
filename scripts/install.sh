#!/bin/sh
set -eu

REPO_URL="${BRANP_REPO_URL:-https://github.com/BrandonPacewic/branp}"
GIT_REF="${BRANP_INSTALL_REF:-}"

say() {
  printf '%s\n' "$1"
}

die() {
  printf 'error: %s\n' "$1" >&2
  exit 1
}

need_cmd() {
  command -v "$1" >/dev/null 2>&1 || die "$1 is required but was not found in PATH"
}

need_cmd git
need_cmd cargo
need_cmd rustc

if ! rustc --version >/dev/null 2>&1; then
  die "rustc is installed but could not run"
fi

if ! cargo --version >/dev/null 2>&1; then
  die "cargo is installed but could not run"
fi

tmpdir="$(mktemp -d 2>/dev/null || mktemp -d -t branp-install)"
cleanup() {
  rm -rf "$tmpdir"
}
trap cleanup EXIT INT TERM

say "Installing branp from source..."
say "Repository: $REPO_URL"

git clone --depth 1 "$REPO_URL" "$tmpdir/branp"

if [ -n "$GIT_REF" ]; then
  say "Checking out ref: $GIT_REF"
  git -C "$tmpdir/branp" fetch --depth 1 origin "$GIT_REF"
  git -C "$tmpdir/branp" checkout FETCH_HEAD
fi

cargo install --path "$tmpdir/branp" --locked

if [ -n "${CARGO_INSTALL_ROOT:-}" ]; then
  install_bin="$CARGO_INSTALL_ROOT/bin"
elif command -v bp >/dev/null 2>&1; then
  install_bin="$(dirname "$(command -v bp)")"
else
  install_bin="${CARGO_HOME:-$HOME/.cargo}/bin"
fi

say "Installed bp to $install_bin"
say "Run 'bp --version' to verify the install."
