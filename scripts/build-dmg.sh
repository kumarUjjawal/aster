#!/bin/bash
# build-dmg.sh - Build DMG installer for Aster
# Usage: ./scripts/build-dmg.sh [arm64|x86_64|universal]

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

APP_NAME="Aster"
BUNDLE_ID="com.kumarujjawal.aster"
VERSION=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')

echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${BLUE}  Building ${APP_NAME} v${VERSION} DMG Installer${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"

# Determine target architecture
ARCH="${1:-$(uname -m)}"

# Map architecture names
case "$ARCH" in
    arm64|aarch64)
        RUST_TARGET="aarch64-apple-darwin"
        DMG_SUFFIX="arm64"
        ;;
    x86_64|intel)
        RUST_TARGET="x86_64-apple-darwin"
        DMG_SUFFIX="x86_64"
        ;;
    universal)
        echo -e "${YELLOW}Building universal binary...${NC}"
        # Build both architectures
        "$0" arm64
        "$0" x86_64
        
        echo -e "\n${BLUE}Creating universal DMG...${NC}"
        # Create universal app by combining both
        ARM_APP="target/aarch64-apple-darwin/release/bundle/osx/${APP_NAME}.app"
        X86_APP="target/x86_64-apple-darwin/release/bundle/osx/${APP_NAME}.app"
        UNIVERSAL_APP="target/universal/${APP_NAME}.app"
        
        rm -rf "target/universal"
        mkdir -p "target/universal"
        cp -R "$ARM_APP" "$UNIVERSAL_APP"
        
        # Create universal binary using lipo
        lipo -create \
            "$ARM_APP/Contents/MacOS/aster" \
            "$X86_APP/Contents/MacOS/aster" \
            -output "$UNIVERSAL_APP/Contents/MacOS/aster"
        
        # Create universal DMG
        create_dmg "target/universal" "universal"
        exit 0
        ;;
    *)
        echo -e "${RED}Error: Unknown architecture '$ARCH'${NC}"
        echo "Usage: $0 [arm64|x86_64|universal]"
        exit 1
        ;;
esac

echo -e "\n${YELLOW}Target: ${RUST_TARGET}${NC}"

# Check for cargo-bundle
if ! command -v cargo-bundle &> /dev/null; then
    echo -e "${YELLOW}Installing cargo-bundle...${NC}"
    cargo install cargo-bundle
fi

# Ensure the toolchain target is installed
echo -e "\n${BLUE}[1/4] Ensuring Rust target is installed...${NC}"
rustup target add "$RUST_TARGET" 2>/dev/null || true

# Build release binary
echo -e "\n${BLUE}[2/4] Building release binary for ${RUST_TARGET}...${NC}"
cargo build --release --target "$RUST_TARGET"

# Create app bundle
echo -e "\n${BLUE}[3/4] Creating macOS app bundle...${NC}"
cargo bundle --release --target "$RUST_TARGET"

# Function to create DMG
create_dmg() {
    local APP_DIR="$1"
    local SUFFIX="$2"
    local DMG_NAME="${APP_NAME}-${SUFFIX}.dmg"
    local APP_PATH="${APP_DIR}/${APP_NAME}.app"
    local DMG_TEMP="target/dmg-temp"
    
    echo -e "\n${BLUE}[4/4] Creating DMG: ${DMG_NAME}...${NC}"
    
    # Clean up any previous temp directory
    rm -rf "$DMG_TEMP"
    mkdir -p "$DMG_TEMP"
    
    # Copy app to temp directory
    cp -R "$APP_PATH" "$DMG_TEMP/"
    
    # Create symbolic link to Applications folder
    ln -s /Applications "$DMG_TEMP/Applications"
    
    # Remove any existing DMG
    rm -f "$DMG_NAME"
    
    # Create the DMG
    hdiutil create \
        -volname "$APP_NAME" \
        -srcfolder "$DMG_TEMP" \
        -ov \
        -format UDZO \
        "$DMG_NAME"
    
    # Clean up temp directory
    rm -rf "$DMG_TEMP"
    
    # Get DMG size
    DMG_SIZE=$(du -h "$DMG_NAME" | cut -f1)
    
    echo -e "${GREEN}✓ Created: ${DMG_NAME} (${DMG_SIZE})${NC}"
}

# Create DMG
BUNDLE_DIR="target/${RUST_TARGET}/release/bundle/osx"
create_dmg "$BUNDLE_DIR" "$DMG_SUFFIX"

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  Build complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
