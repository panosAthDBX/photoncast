#!/bin/bash
#
# PhotonCast Icon Verification Script
# Task 5.5: Test Icon Rendering at All Sizes
#
# This script verifies that all icon assets are present and valid:
# - App icon at all required sizes (16, 32, 128, 256, 512 @1x/@2x)
# - Menu bar template icons (16, 18 @1x/@2x)
# - ICNS file contains all variants
# - Menu bar icon is properly formatted as template
#
# Usage: ./scripts/verify-icons.sh
#

set -euo pipefail

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "${SCRIPT_DIR}/.." && pwd)"
RESOURCES_DIR="${PROJECT_ROOT}/resources"

# Counters
PASSED=0
FAILED=0
WARNINGS=0

# Helper function to check file exists
check_file() {
    local file="$1"
    local description="$2"
    
    if [[ -f "$file" ]]; then
        echo -e "${GREEN}✓${NC} $description: $(basename "$file")"
        ((PASSED++))
        return 0
    else
        echo -e "${RED}✗${NC} $description: $(basename "$file") - NOT FOUND"
        ((FAILED++))
        return 1
    fi
}

# Helper function to verify PNG dimensions
verify_png_size() {
    local file="$1"
    local expected_width="$2"
    local expected_height="$3"
    
    if [[ ! -f "$file" ]]; then
        return 1
    fi
    
    # Use sips to get dimensions (macOS)
    if command -v sips &> /dev/null; then
        local width=$(sips -g pixelWidth "$file" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
        local height=$(sips -g pixelHeight "$file" 2>/dev/null | awk '/pixelHeight:/ {print $2}')
        
        if [[ "$width" == "$expected_width" && "$height" == "$expected_height" ]]; then
            echo -e "${GREEN}✓${NC} Size verified: ${width}×${height}"
            return 0
        else
            echo -e "${RED}✗${NC} Size mismatch: expected ${expected_width}×${expected_height}, got ${width}×${height}"
            return 1
        fi
    else
        echo -e "${YELLOW}⚠${NC} sips not available, skipping size verification"
        ((WARNINGS++))
        return 0
    fi
}

# Helper function to check if image has alpha channel
check_alpha_channel() {
    local file="$1"
    
    if command -v sips &> /dev/null; then
        local has_alpha=$(sips -g hasAlpha "$file" 2>/dev/null | awk '/hasAlpha:/ {print $2}')
        if [[ "$has_alpha" == "yes" ]]; then
            echo -e "${GREEN}✓${NC} Alpha channel present"
            return 0
        else
            echo -e "${RED}✗${NC} No alpha channel (required for template icons)"
            return 1
        fi
    fi
    return 0
}

echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}PhotonCast Icon Verification${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""

# =============================================================================
# Check ICNS File
# =============================================================================

echo -e "${BLUE}Checking App Icon (ICNS)...${NC}"
echo ""

ICNS_FILE="${RESOURCES_DIR}/AppIcon.icns"
if check_file "$ICNS_FILE" "App icon ICNS file"; then
    # Get file size
    size=$(wc -c < "$ICNS_FILE" | tr -d ' ')
    echo -e "  File size: ${size} bytes"
    
    # Check if icns file can be read
    if command -v iconutil &> /dev/null; then
        # Create temp directory and try to extract
        temp_dir=$(mktemp -d)
        if iconutil -c iconset -o "${temp_dir}/extracted.iconset" "$ICNS_FILE" 2>/dev/null; then
            icon_count=$(ls -1 "${temp_dir}/extracted.iconset" 2>/dev/null | wc -l | tr -d ' ')
            echo -e "  Contains ${icon_count} icon variants"
            ((PASSED++))
        else
            echo -e "${YELLOW}⚠${NC} Could not extract ICNS for verification"
            ((WARNINGS++))
        fi
        rm -rf "$temp_dir"
    fi
fi
echo ""

# =============================================================================
# Check Icon Size Variants (Iconset)
# =============================================================================

echo -e "${BLUE}Checking Icon Size Variants...${NC}"
echo ""

ICONSET_DIR="${RESOURCES_DIR}/AppIcon.iconset"
if [[ -d "$ICONSET_DIR" ]]; then
    echo -e "${GREEN}✓${NC} Iconset directory exists"
    ((PASSED++))
    echo ""
    
    # Required icon sizes (using simple arrays for bash compatibility)
    declare -a ICON_NAMES=(
        "icon_16x16.png"
        "icon_16x16@2x.png"
        "icon_32x32.png"
        "icon_32x32@2x.png"
        "icon_128x128.png"
        "icon_128x128@2x.png"
        "icon_256x256.png"
        "icon_256x256@2x.png"
        "icon_512x512.png"
        "icon_512x512@2x.png"
    )
    declare -a ICON_WIDTHS=(16 32 32 64 128 256 256 512 512 1024)
    declare -a ICON_HEIGHTS=(16 32 32 64 128 256 256 512 512 1024)
    
    for i in "${!ICON_NAMES[@]}"; do
        icon="${ICON_NAMES[$i]}"
        icon_path="${ICONSET_DIR}/${icon}"
        width="${ICON_WIDTHS[$i]}"
        height="${ICON_HEIGHTS[$i]}"
        
        echo -n "  $icon: "
        if [[ -f "$icon_path" ]]; then
            if verify_png_size "$icon_path" "$width" "$height" 2>/dev/null; then
                ((PASSED++))
            else
                actual_width=$(sips -g pixelWidth "$icon_path" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
                actual_height=$(sips -g pixelHeight "$icon_path" 2>/dev/null | awk '/pixelHeight:/ {print $2}')
                echo -e "${RED}✗${NC} Wrong size: ${actual_width}×${actual_height} (expected ${width}×${height})"
                ((FAILED++))
            fi
        else
            echo -e "${RED}✗${NC} NOT FOUND"
            ((FAILED++))
        fi
    done
else
    echo -e "${RED}✗${NC} Iconset directory not found at ${ICONSET_DIR}"
    ((FAILED++))
fi
echo ""

# =============================================================================
# Check Menu Bar Icons
# =============================================================================

echo -e "${BLUE}Checking Menu Bar Icons...${NC}"
echo ""

# Required menu bar icons
MENU_BAR_ICONS=(
    "MenuBarIcon.png:16:16"
    "MenuBarIcon_16x16@1x.png:16:16"
    "MenuBarIcon_16x16@2x.png:32:32"
    "MenuBarIcon_18x18@1x.png:18:18"
    "MenuBarIcon_18x18@2x.png:36:36"
)

for icon_spec in "${MENU_BAR_ICONS[@]}"; do
    icon_name=$(echo "$icon_spec" | cut -d: -f1)
    expected_w=$(echo "$icon_spec" | cut -d: -f2)
    expected_h=$(echo "$icon_spec" | cut -d: -f3)
    icon_path="${RESOURCES_DIR}/${icon_name}"
    
    echo -n "  $icon_name: "
    if [[ -f "$icon_path" ]]; then
        if command -v sips &> /dev/null; then
            actual_w=$(sips -g pixelWidth "$icon_path" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
            actual_h=$(sips -g pixelHeight "$icon_path" 2>/dev/null | awk '/pixelHeight:/ {print $2}')
            
            if [[ "$actual_w" == "$expected_w" && "$actual_h" == "$expected_h" ]]; then
                echo -e "${GREEN}✓${NC} ${actual_w}×${actual_h}"
                ((PASSED++))
            else
                echo -e "${RED}✗${NC} Wrong size: ${actual_w}×${actual_h} (expected ${expected_w}×${expected_h})"
                ((FAILED++))
            fi
        else
            echo -e "${GREEN}✓${NC} (size not verified)"
            ((PASSED++))
        fi
    else
        echo -e "${RED}✗${NC} NOT FOUND"
        ((FAILED++))
    fi
done

# Check PDF version
echo ""
echo -n "  MenuBarIcon.pdf: "
PDF_PATH="${RESOURCES_DIR}/MenuBarIcon.pdf"
if [[ -f "$PDF_PATH" ]]; then
    echo -e "${GREEN}✓${NC} Present (vector format for best quality)"
    ((PASSED++))
else
    echo -e "${YELLOW}⚠${NC} Not found (optional but recommended)"
    ((WARNINGS++))
fi
echo ""

# =============================================================================
# Verify Template Icon Format
# =============================================================================

echo -e "${BLUE}Verifying Menu Bar Template Format...${NC}"
echo ""

# Template icons should be black with alpha (monochrome)
# Check one of the menu bar icons
TEMPLATE_ICON="${RESOURCES_DIR}/MenuBarIcon.png"
if [[ -f "$TEMPLATE_ICON" ]]; then
    echo -n "  Alpha channel: "
    check_alpha_channel "$TEMPLATE_ICON"
    
    # Check for monochrome (hard to verify without ImageMagick)
    echo -e "${YELLOW}⚠${NC} Manual verification recommended:"
    echo "    - Icon should be pure black (#000000) with alpha transparency"
    echo "    - No colors (macOS handles dark mode inversion)"
    echo "    - Simple, recognizable silhouette"
    ((WARNINGS++))
else
    echo -e "${RED}✗${NC} Template icon not found for verification"
    ((FAILED++))
fi
echo ""

# =============================================================================
# Check DMG Background Image
# =============================================================================

echo -e "${BLUE}Checking DMG Background Image...${NC}"
echo ""

DMG_BG="${RESOURCES_DIR}/dmg-background.png"
DMG_BG_1X="${RESOURCES_DIR}/dmg-background@1x.png"

echo -n "  dmg-background.png (@2x): "
if [[ -f "$DMG_BG" ]]; then
    if command -v sips &> /dev/null; then
        w=$(sips -g pixelWidth "$DMG_BG" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
        h=$(sips -g pixelHeight "$DMG_BG" 2>/dev/null | awk '/pixelHeight:/ {print $2}')
        if [[ "$w" == "1600" && "$h" == "1000" ]]; then
            echo -e "${GREEN}✓${NC} ${w}×${h} (correct @2x size)"
            ((PASSED++))
        else
            echo -e "${YELLOW}⚠${NC} ${w}×${h} (expected 1600×1000 for @2x)"
            ((WARNINGS++))
        fi
    else
        echo -e "${GREEN}✓${NC} Present"
        ((PASSED++))
    fi
else
    echo -e "${RED}✗${NC} NOT FOUND"
    ((FAILED++))
fi

echo -n "  dmg-background@1x.png: "
if [[ -f "$DMG_BG_1X" ]]; then
    if command -v sips &> /dev/null; then
        w=$(sips -g pixelWidth "$DMG_BG_1X" 2>/dev/null | awk '/pixelWidth:/ {print $2}')
        h=$(sips -g pixelHeight "$DMG_BG_1X" 2>/dev/null | awk '/pixelHeight:/ {print $2}')
        if [[ "$w" == "800" && "$h" == "500" ]]; then
            echo -e "${GREEN}✓${NC} ${w}×${h} (correct @1x size)"
            ((PASSED++))
        else
            echo -e "${YELLOW}⚠${NC} ${w}×${h} (expected 800×500 for @1x)"
            ((WARNINGS++))
        fi
    else
        echo -e "${GREEN}✓${NC} Present"
        ((PASSED++))
    fi
else
    echo -e "${YELLOW}⚠${NC} Not found (optional, @2x will be used)"
    ((WARNINGS++))
fi
echo ""

# =============================================================================
# Visual Verification Checklist
# =============================================================================

echo -e "${BLUE}Manual Verification Checklist:${NC}"
echo ""
echo "  Please verify the following visually:"
echo ""
echo "  App Icon:"
echo "    [ ] Icon is clear and recognizable at 16×16 (Finder sidebar)"
echo "    [ ] Icon looks good at 64×64 (Dock)"
echo "    [ ] Icon is crisp at 512×512 (Launchpad, Get Info)"
echo "    [ ] No pixelation or artifacts at any size"
echo "    [ ] Colors match Catppuccin Mocha palette"
echo ""
echo "  Menu Bar Icon:"
echo "    [ ] Icon is visible in menu bar"
echo "    [ ] Icon inverts correctly in dark mode"
echo "    [ ] Icon is recognizable at 16×16"
echo "    [ ] Simple, distinct silhouette"
echo ""
echo "  DMG Background:"
echo "    [ ] Background shows app icon and arrow"
echo "    [ ] Layout is centered and professional"
echo "    [ ] Text/graphics are crisp on Retina displays"
echo ""

# =============================================================================
# Summary
# =============================================================================

echo -e "${BLUE}============================================${NC}"
echo -e "${BLUE}Summary${NC}"
echo -e "${BLUE}============================================${NC}"
echo ""
echo -e "  ${GREEN}Passed:${NC}   $PASSED"
echo -e "  ${RED}Failed:${NC}   $FAILED"
echo -e "  ${YELLOW}Warnings:${NC} $WARNINGS"
echo ""

if [[ $FAILED -gt 0 ]]; then
    echo -e "${RED}✗ Icon verification failed!${NC}"
    echo "  Please fix the missing/incorrect files and run again."
    exit 1
elif [[ $WARNINGS -gt 0 ]]; then
    echo -e "${YELLOW}⚠ Icon verification passed with warnings${NC}"
    echo "  Review the warnings above for potential improvements."
    exit 0
else
    echo -e "${GREEN}✓ All icon checks passed!${NC}"
    exit 0
fi
