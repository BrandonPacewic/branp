#!/usr/bin/env bash

set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null && pwd )"
ROOT="$DIR/../"
cd $ROOT

RED='\033[0;31m'
GREEN='\033[0;32m'
NC='\033[0m'

FAILED=0
FAST=0
RUST_TOOLCHAIN="$(awk -F '"' '/^[[:space:]]*channel[[:space:]]*=/{print $2; exit}' rust-toolchain.toml)"

if [[ -z "$RUST_TOOLCHAIN" ]]; then
	echo "Unable to read Rust toolchain from rust-toolchain.toml"
	exit 1
fi

while [[ $# -gt 0 ]]; do
	case $1 in
		--fast)
			FAST=1
			shift
			;;
		*)
			echo "Unknown option: $1"
			echo "Usage: $0 [--fast]"
			echo "  --fast         skip build (format + file checks only)"
			exit 1
			;;
	esac
done

function run_check() {
	local name="$1"
	shift

	local name_len=${#name}
	local dots_len=$((46 - name_len))
	local dots=$(printf '%*s' "$dots_len" | tr ' ' '.')

	printf "%s%s" "$name" "$dots"

	set +e
	log=$("$@" 2>&1)
	result=$?
	set -e

	if [[ $result -eq 0 ]]; then
		echo -e "[${GREEN}ok${NC}]"
	else
		echo -e "[${RED}FAIL${NC}]"
		echo "$log"
		FAILED=1
	fi
}

function ensure_rust_toolchain() {
	if ! command -v rustup >/dev/null 2>&1; then
		echo "rustup is required so local and CI use Rust $RUST_TOOLCHAIN from rust-toolchain.toml"
		return 1
	fi

	if ! rustup run "$RUST_TOOLCHAIN" rustc --version >/dev/null 2>&1 \
		|| ! rustup component list --toolchain "$RUST_TOOLCHAIN" --installed | grep -q '^clippy-' \
		|| ! rustup component list --toolchain "$RUST_TOOLCHAIN" --installed | grep -q '^rustfmt-'; then
		rustup toolchain install "$RUST_TOOLCHAIN" --component clippy --component rustfmt
	fi
}

function pinned_cargo() {
	local rustfmt
	rustfmt="$(rustup which --toolchain "$RUST_TOOLCHAIN" rustfmt)"
	RUSTFMT="$rustfmt" rustup run "$RUST_TOOLCHAIN" cargo "$@"
}

function pinned_rustc() {
	rustup run "$RUST_TOOLCHAIN" rustc "$@"
}

function check_rust_version() {
	local actual
	actual="$(pinned_rustc --version)"

	if [[ "$RUST_TOOLCHAIN" == nightly* ]]; then
		if [[ "$actual" != rustc\ *-nightly* ]]; then
			echo "Expected nightly rustc from rust-toolchain.toml, got: $actual"
			return 1
		fi
	elif [[ "$actual" != rustc\ "$RUST_TOOLCHAIN"* ]]; then
		echo "Expected rustc $RUST_TOOLCHAIN from rust-toolchain.toml, got: $actual"
		return 1
	fi

	echo "$actual"
	pinned_cargo --version
}

function check_large_files() {
	local limit=$((250 * 1024))
	local found=0
	while IFS= read -r file; do
		local size
		size=$(wc -c < "$file")
		if [[ "$size" -gt "$limit" ]]; then
			echo "Large file ($(( size / 1024 ))KB): $file"
			found=1
		fi
	done < <(git ls-files)
	return "$found"
}

function check_default_branch() {
	[[ "$(git rev-parse --abbrev-ref HEAD)" != mega ]]
}

function check_merge_conflicts() {
	! git ls-files | xargs grep -lE '^(<{7}|={7}|>{7})' 2>/dev/null | grep -q .
}

run_check "rust_toolchain" ensure_rust_toolchain
run_check "rust_version" check_rust_version
run_check "check_default_branch" check_default_branch
run_check "cargo_fmt" pinned_cargo fmt --all -- --check
run_check "cargo_clippy" pinned_cargo clippy --workspace --all-targets --all-features -- -D warnings
run_check "check_large_files" check_large_files
run_check "check_merge_conflicts" check_merge_conflicts

if [[ "$FAST" -eq 0 ]]; then
	run_check "build" pinned_cargo build --workspace
fi

echo ""
if [[ $FAILED -eq 1 ]]; then
	echo -e "${RED}Lint failed${NC}"
	exit 1
else
	echo -e "${GREEN}All checks passed${NC}"
fi
