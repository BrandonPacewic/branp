#!/bin/sh

BINARY_NAME="branp-cli"
INSTALL_DIR="$HOME/.cargo/bin"
BINARY_PATH="target/release/$BINARY_NAME"

cargo build --release
mkdir -p "$INSTALL_DIR"

if [ -f "$INSTALL_DIR/bp" ]; then
    rm "$INSTALL_DIR/bp"
fi

cp "$BINARY_PATH" "$INSTALL_DIR/bp"

echo "Installation complete!"
