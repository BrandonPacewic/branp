#!/usr/bin/env bash

set -e

DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" >/dev/null && pwd )"
ROOT="$DIR/../"

cp "$DIR/pre-commit" "$ROOT/.git/hooks/pre-commit"
chmod +x "$ROOT/.git/hooks/pre-commit"

echo "Pre-commit hook installed."
