#!/bin/bash
#
# PhotonCast Icon Generation Script
# Generates all required app icon sizes from source and compiles ICNS file
#
# Usage: ./scripts/generate-icons.sh
#

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
RESOURCES_DIR="$PROJECT_ROOT/resources"
ASSETS_DIR="$PROJECT_ROOT/assets"
ICONSET_DIR="$RESOURCES_DIR/AppIcon.iconset"

# Source icon
SOURCE_ICON="$ASSETS_DIR/icon-source-1024.png"

# Colors (Catppuccin Mocha)
MAUVE="#cba6f7"
PINK="#f5c2e7"
BASE="#1e1e2e"

echo "🎨 PhotonCast Icon Generator"
echo "============================="
echo ""

# Check for source icon
if [ ! -f "$SOURCE_ICON" ]; then
    echo "❌ Source icon not found: $SOURCE_ICON"
    echo "   Run: python3 scripts/generate-icon-source.py"
    exit 1
fi

echo "✓ Source icon found: $SOURCE_ICON"
echo ""

# Create iconset directory
mkdir -p "$ICONSET_DIR"

# Define all required sizes
# Format: "name size scale"
declare -a ICON_SIZES=(
    "16x16 16 1"
    "16x16@2x 32 2"
    "32x32 32 1"
    "32x32@2x 64 2"
    "128x128 128 1"
    "128x128@2x 256 2"
    "256x256 256 1"
    "256x256@2x 512 2"
    "512x512 512 1"
    "512x512@2x 1024 2"
)

echo "📐 Generating icon sizes..."

# Generate each size using sips (macOS built-in tool)
for spec in "${ICON_SIZES[@]}"; do
    read -r name size scale <<< "$spec"
    output_file="$ICONSET_DIR/icon_$name.png"
    pixel_size=$((size * scale))

    echo "  → icon_$name.png (${pixel_size}×${pixel_size}px)"

    # Use sips to resize (macOS native, no ImageMagick needed)
    sips -z "$pixel_size" "$pixel_size" "$SOURCE_ICON" --out "$output_file" > /dev/null 2>&1

    if [ $? -ne 0 ]; then
        echo "    ⚠️  sips failed, trying Python fallback..."
        python3 -c "
from PIL import Image
img = Image.open('$SOURCE_ICON')
img = img.resize(($pixel_size, $pixel_size), Image.Resampling.LANCZOS)
img.save('$output_file', 'PNG')
"
    fi
done

echo ""
echo "📦 Building ICNS file..."

# Build ICNS using iconutil
cd "$RESOURCES_DIR"

# Remove old ICNS if exists
rm -f AppIcon.icns

# Create ICNS from iconset
iconutil -c icns AppIcon.iconset

if [ $? -eq 0 ]; then
    echo "✓ Created: $RESOURCES_DIR/AppIcon.icns"
else
    echo "❌ Failed to create ICNS file"
    exit 1
fi

# Verify ICNS content
echo ""
echo "🔍 Verifying ICNS contents..."
iconutil -l AppIcon.icns 2>/dev/null || echo "  (iconutil listing not available)"

# List generated files
echo ""
echo "📁 Generated files:"
echo "  AppIcon.icns: $(ls -lh AppIcon.icns | awk '{print $5}')"
echo ""
echo "  Iconset contents:"
ls -la AppIcon.iconset/ | grep -E "icon_" | awk '{print "    " $9 " (" $5 ")"}'

echo ""
echo "✅ Icon generation complete!"
echo "   Install icon to app bundle:"
echo "   cp $RESOURCES_DIR/AppIcon.icns PhotonCast.app/Contents/Resources/"

# Also copy menu bar icons to expected location
echo ""
echo "📋 Menu bar icons ready at:"
ls -1 "$RESOURCES_DIR"/MenuBarIcon*.png 2>/dev/null | while read f; do
    echo "   $(basename "$f")"
done

# DMG background
echo ""
echo "🖼️  DMG background ready:"
ls -1 "$RESOURCES_DIR"/dmg-background*.png 2>/dev/null | while read f; do
    echo "   $(basename "$f") ($(stat -f%z "$f" 2>/dev/null || stat -c%s "$f" 2>/dev/null | numfmt --to=iec 2>/dev/null || echo "unknown"))"
done

exit 0
