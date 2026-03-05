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
	local cmd="$2"

	local name_len=${#name}
	local dots_len=$((46 - name_len))
	local dots=$(printf '%*s' "$dots_len" | tr ' ' '.')

	printf "%s%s" "$name" "$dots"

	set +e
	log=$(eval "$cmd" 2>&1)
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

run_check "check_default_branch" "[[ \$(git rev-parse --abbrev-ref HEAD) != mega ]]"
run_check "cargo_fmt" "cargo fmt --all -- --check"
run_check "cargo_clippy" "cargo clippy --all-targets --all-features -- -D warnings"
run_check "check_large_files" "check_large_files"
run_check "check_merge_conflicts" "! git ls-files | xargs grep -lP '^(<{7}|={7}|>{7})' 2>/dev/null | grep -q ."

if [[ "$FAST" -eq 0 ]]; then
	run_check "build" "cargo build"
fi

echo ""
if [[ $FAILED -eq 1 ]]; then
	echo -e "${RED}Lint failed${NC}"
	exit 1
else
	echo -e "${GREEN}All checks passed${NC}"
fi
