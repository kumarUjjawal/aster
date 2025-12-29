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

# Get cargo target directory (respects CARGO_TARGET_DIR and .cargo/config.toml)
get_target_dir() {
    # Check if cargo can tell us
    if command -v cargo &> /dev/null; then
        local target_dir=$(cargo metadata --format-version=1 2>/dev/null | grep -o '"target_directory":"[^"]*"' | cut -d'"' -f4)
        if [ -n "$target_dir" ]; then
            echo "$target_dir"
            return
        fi
    fi
    # Fallback to default
    echo "target"
}

TARGET_DIR=$(get_target_dir)
echo -e "${YELLOW}Target directory: ${TARGET_DIR}${NC}"

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
        
        # Find the app bundles
        ARM_APP=$(find "$TARGET_DIR" -path "*aarch64-apple-darwin/release/bundle/osx/${APP_NAME}.app" -type d 2>/dev/null | head -1)
        X86_APP=$(find "$TARGET_DIR" -path "*x86_64-apple-darwin/release/bundle/osx/${APP_NAME}.app" -type d 2>/dev/null | head -1)
        
        if [ -z "$ARM_APP" ] || [ -z "$X86_APP" ]; then
            echo -e "${RED}Error: Could not find both architecture builds${NC}"
            exit 1
        fi
        
        UNIVERSAL_DIR="${TARGET_DIR}/universal"
        UNIVERSAL_APP="${UNIVERSAL_DIR}/${APP_NAME}.app"
        
        rm -rf "$UNIVERSAL_DIR"
        mkdir -p "$UNIVERSAL_DIR"
        cp -R "$ARM_APP" "$UNIVERSAL_APP"
        
        # Create universal binary using lipo
        lipo -create \
            "$ARM_APP/Contents/MacOS/aster" \
            "$X86_APP/Contents/MacOS/aster" \
            -output "$UNIVERSAL_APP/Contents/MacOS/aster"
        
        # Create universal DMG
        create_dmg "$UNIVERSAL_DIR" "universal"
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
    local DMG_TEMP="${TARGET_DIR}/dmg-temp"
    
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

# Find the bundle directory (handles custom CARGO_TARGET_DIR)
BUNDLE_DIR=$(find "$TARGET_DIR" -path "*${RUST_TARGET}/release/bundle/osx" -type d 2>/dev/null | head -1)

if [ -z "$BUNDLE_DIR" ] || [ ! -d "$BUNDLE_DIR/${APP_NAME}.app" ]; then
    echo -e "${RED}Error: Could not find app bundle at expected location${NC}"
    echo "Searched in: ${TARGET_DIR}"
    exit 1
fi

echo -e "${YELLOW}Found bundle at: ${BUNDLE_DIR}${NC}"

# Create DMG
create_dmg "$BUNDLE_DIR" "$DMG_SUFFIX"

echo -e "\n${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${GREEN}  Build complete!${NC}"
echo -e "${GREEN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
