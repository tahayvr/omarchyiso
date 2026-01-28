#!/bin/bash
set -e

REPO="tahayvr/omarchyiso"
BINARY_NAME="omarchyiso"
INSTALL_DIR="/usr/local/bin"

GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m'

# Detect Architecture
ARCH=$(uname -m)
case $ARCH in
    x86_64)
        ;;
    aarch64|arm64)
        ARCH="aarch64"
        ;;
    *)
        echo "Error: Unsupported architecture $ARCH"
        exit 1
        ;;
esac

echo -e "${GREEN}Installing OmarchyISO${NC}"

# 1. Check dependencies
if ! command -v curl &> /dev/null; then
    echo -e "${RED}Error: curl is required.${NC}"
    exit 1
fi

# 2. Get latest release URL for specific target
echo "Fetching latest version for $ARCH"
LATEST_URL=$(curl -s "https://api.github.com/repos/$REPO/releases/latest" | \
    grep "browser_download_url" | \
    grep "omarchyiso-$ARCH-linux.tar.gz" | \
    cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
    echo "Error: Could not find release asset for $ARCH."
    exit 1
fi

# 3. Download and Extract
TMP_DIR=$(mktemp -d)
cleanup() {
    rm -rf "$TMP_DIR"
}
trap cleanup EXIT INT TERM

curl -L --fail "$LATEST_URL" -o "$TMP_DIR/omarchyiso.tar.gz"

tar -xzf "$TMP_DIR/omarchyiso.tar.gz" -C "$TMP_DIR"

# 4. Install
echo "Installing to $INSTALL_DIR"
if [ -f "$TMP_DIR/$BINARY_NAME" ]; then
    sudo mv "$TMP_DIR/$BINARY_NAME" "$INSTALL_DIR/"
    sudo chmod +x "$INSTALL_DIR/$BINARY_NAME"
    echo -e "${GREEN}Done! Run '$BINARY_NAME' to start making your custom Omarchy ISO.${NC}"
else
    echo -e "${RED}Error: Binary not found.${NC}"
    exit 1
fi
