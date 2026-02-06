#!/usr/bin/env python3
"""
Generate PhotonCast App Icon - "Photon Beam" Concept
Catppuccin Mocha Palette
- Base: #1e1e2e (background)
- Mauve: #cba6f7 (gradient start)
- Pink: #f5c2e7 (gradient end)
- Flamingo: #f2cdcd (accent highlight)
- Surface0: #313244 (inner glow)
"""

from PIL import Image, ImageDraw, ImageFilter
import math
import os

# Catppuccin Mocha Colors
COLORS = {
    'base': '#1e1e2e',
    'mantle': '#181825',
    'crust': '#11111b',
    'surface0': '#313244',
    'surface1': '#45475a',
    'surface2': '#585b70',
    'overlay0': '#6c7086',
    'overlay1': '#7f849c',
    'text': '#cdd6f4',
    'subtext0': '#a6adc8',
    'subtext1': '#bac2de',
    'mauve': '#cba6f7',
    'pink': '#f5c2e7',
    'flamingo': '#f2cdcd',
    'lavender': '#b4befe',
    'blue': '#89b4fa',
}

def hex_to_rgb(hex_color):
    """Convert hex color to RGB tuple."""
    hex_color = hex_color.lstrip('#')
    return tuple(int(hex_color[i:i+2], 16) for i in (0, 2, 4))

def create_rounded_rectangle_mask(size, radius):
    """Create a rounded rectangle mask."""
    mask = Image.new('L', size, 0)
    draw = ImageDraw.Draw(mask)
    draw.rounded_rectangle((0, 0, size[0], size[1]), radius=radius, fill=255)
    return mask

def create_gradient(size, color1, color2, direction='diagonal'):
    """Create a gradient image."""
    img = Image.new('RGB', size)
    pixels = img.load()
    c1 = hex_to_rgb(color1)
    c2 = hex_to_rgb(color2)
    
    for y in range(size[1]):
        for x in range(size[0]):
            if direction == 'diagonal':
                ratio = (x + y) / (size[0] + size[1])
            elif direction == 'horizontal':
                ratio = x / size[0]
            else:  # vertical
                ratio = y / size[1]
            
            r = int(c1[0] + (c2[0] - c1[0]) * ratio)
            g = int(c1[1] + (c2[1] - c1[1]) * ratio)
            b = int(c1[2] + (c2[2] - c1[2]) * ratio)
            pixels[x, y] = (r, g, b)
    
    return img

def create_photon_beam_icon(size=1024):
    """Create the Photon Beam app icon."""
    # Create base image with transparency
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    
    # Calculate dimensions
    padding = int(size * 0.05)  # 5% padding
    icon_size = size - (padding * 2)
    corner_radius = int(icon_size * 0.22)  # ~22% corner radius per spec
    
    # Create background with rounded corners
    bg_color = hex_to_rgb(COLORS['base'])
    bg = Image.new('RGBA', (icon_size, icon_size), bg_color + (255,))
    
    # Create rounded mask for the icon shape
    mask = create_rounded_rectangle_mask((icon_size, icon_size), corner_radius)
    
    # Apply rounded corners to background
    bg.putalpha(mask)
    
    # Paste background onto main image
    img.paste(bg, (padding, padding), bg)
    
    # Create inner gradient overlay for "liquid glass" effect
    gradient = create_gradient((icon_size, icon_size), COLORS['mauve'], COLORS['pink'], 'diagonal')
    gradient = gradient.convert('RGBA')
    
    # Create inner content area (slightly smaller than background)
    inner_padding = int(icon_size * 0.08)
    inner_size = icon_size - (inner_padding * 2)
    inner_radius = int(inner_size * 0.18)
    
    # Draw photon beam design
    draw = ImageDraw.Draw(img)
    
    # Center coordinates
    center_x = size // 2
    center_y = size // 2
    
    # Create the photon beam effect - multiple horizontal lines with glow
    beam_y_start = center_y - int(size * 0.15)
    beam_height = int(size * 0.08)
    beam_spacing = int(size * 0.04)
    beam_width = int(size * 0.5)
    
    # Beam colors with gradient
    beam_colors = [
        COLORS['flamingo'],
        COLORS['pink'],
        COLORS['mauve'],
        COLORS['lavender'],
    ]
    
    # Draw beam lines with rounded ends
    for i, color in enumerate(beam_colors):
        y_offset = beam_y_start + (i * (beam_height + beam_spacing))
        
        # Main beam line
        x1 = center_x - beam_width // 2
        y1 = y_offset
        x2 = center_x + beam_width // 2
        y2 = y_offset + beam_height
        
        # Draw with rounded corners
        radius = beam_height // 2
        
        # Draw the beam with glow effect
        for glow in range(3, 0, -1):
            alpha = int(80 / glow)
            glow_color = hex_to_rgb(color) + (alpha,)
            glow_draw = ImageDraw.Draw(img)
            glow_padding = glow * 2
            glow_draw.rounded_rectangle(
                [x1 - glow_padding, y1 - glow_padding, x2 + glow_padding, y2 + glow_padding],
                radius=radius + glow_padding,
                fill=glow_color
            )
        
        # Draw main beam
        beam_rgb = hex_to_rgb(color)
        draw.rounded_rectangle([x1, y1, x2, y2], radius=radius, fill=beam_rgb + (255,))
    
    # Add a subtle glow dot (the "photon" source)
    dot_radius = int(size * 0.06)
    dot_x = center_x - beam_width // 2 - int(size * 0.08)
    dot_y = center_y
    
    # Draw glow around dot
    for glow in range(4, 0, -1):
        alpha = int(60 / glow)
        glow_size = dot_radius + (glow * 3)
        draw.ellipse(
            [dot_x - glow_size, dot_y - glow_size, dot_x + glow_size, dot_y + glow_size],
            fill=hex_to_rgb(COLORS['flamingo']) + (alpha,)
        )
    
    # Draw main dot
    draw.ellipse(
        [dot_x - dot_radius, dot_y - dot_radius, dot_x + dot_radius, dot_y + dot_radius],
        fill=hex_to_rgb(COLORS['flamingo']) + (255,)
    )
    
    # Add subtle inner shadow/highlight for depth
    highlight = Image.new('RGBA', (icon_size, icon_size), (255, 255, 255, 30))
    highlight_mask = create_rounded_rectangle_mask((icon_size, icon_size), corner_radius)
    highlight.putalpha(highlight_mask)
    
    # Apply slight blur for glass effect
    img = img.filter(ImageFilter.GaussianBlur(radius=0.5))
    
    # Add highlight at top
    highlight_draw = ImageDraw.Draw(highlight)
    highlight_height = int(icon_size * 0.15)
    top_gradient = Image.new('RGBA', (icon_size, highlight_height))
    for y in range(highlight_height):
        alpha = int(40 * (1 - y / highlight_height))
        for x in range(icon_size):
            top_gradient.putpixel((x, y), (255, 255, 255, alpha))
    
    # Paste highlight
    img.paste(highlight, (padding, padding), highlight)
    
    return img

def create_menu_bar_icon(size=16):
    """Create simplified monochrome template icon for menu bar."""
    # Create image with transparency
    img = Image.new('RGBA', (size, size), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    
    # Use black for template image (macOS will invert automatically)
    black = (0, 0, 0, 255)
    
    # Padding for 1-2px margin
    padding = max(1, size // 8)
    
    # Draw simplified photon beam - just 3 horizontal lines
    beam_height = max(1, size // 8)
    beam_spacing = max(2, size // 6)
    beam_width = size - (padding * 2) - 2
    
    start_x = padding + 1
    center_y = size // 2
    
    # Draw 3 horizontal lines (simplified beam)
    for i in range(-1, 2):
        y = center_y + (i * beam_spacing) - (beam_height // 2)
        # Ensure within bounds
        if y >= padding and y + beam_height <= size - padding:
            draw.rounded_rectangle(
                [start_x, y, start_x + beam_width, y + beam_height],
                radius=beam_height // 2,
                fill=black
            )
    
    # Add small dot for photon source
    dot_size = max(2, size // 6)
    dot_x = start_x + dot_size
    dot_y = center_y
    draw.ellipse(
        [dot_x - dot_size, dot_y - dot_size, dot_x + dot_size, dot_y + dot_size],
        fill=black
    )
    
    return img

def create_dmg_background(width=1600, height=1000):
    """Create DMG background image with drag-to-Applications design."""
    # Create base image with Catppuccin dark theme
    img = Image.new('RGBA', (width, height), hex_to_rgb(COLORS['base']) + (255,))
    draw = ImageDraw.Draw(img)
    
    # Add subtle gradient overlay
    for y in range(height):
        ratio = y / height
        alpha = int(20 * (1 - ratio))
        draw.line([(0, y), (width, y)], fill=hex_to_rgb(COLORS['mantle']) + (alpha,))
    
    # App icon position (left side)
    icon_size = 256
    icon_x = width // 4 - icon_size // 2
    icon_y = height // 2 - icon_size // 2
    
    # Generate and place app icon
    app_icon = create_photon_beam_icon(512)
    app_icon = app_icon.resize((icon_size, icon_size), Image.Resampling.LANCZOS)
    img.paste(app_icon, (icon_x, icon_y), app_icon)
    
    # Draw arrow pointing right
    arrow_x_start = icon_x + icon_size + 60
    arrow_x_end = width // 2 + 100
    arrow_y = height // 2
    
    # Arrow body
    arrow_width = 8
    arrow_head_size = 30
    
    # Draw arrow line
    draw.rounded_rectangle(
        [arrow_x_start, arrow_y - arrow_width//2, arrow_x_end, arrow_y + arrow_width//2],
        radius=arrow_width//2,
        fill=hex_to_rgb(COLORS['text']) + (200,)
    )
    
    # Draw arrow head (triangle)
    arrow_head = [
        (arrow_x_end, arrow_y - arrow_head_size),
        (arrow_x_end + arrow_head_size, arrow_y),
        (arrow_x_end, arrow_y + arrow_head_size),
    ]
    draw.polygon(arrow_head, fill=hex_to_rgb(COLORS['text']) + (200,))
    
    # Applications folder representation (right side)
    folder_x = width * 3 // 4 - 100
    folder_y = height // 2 - 80
    folder_width = 200
    folder_height = 160
    
    # Draw folder icon
    folder_color = hex_to_rgb(COLORS['surface1'])
    
    # Folder tab
    tab_width = 60
    tab_height = 25
    draw.rounded_rectangle(
        [folder_x, folder_y, folder_x + tab_width, folder_y + tab_height],
        radius=8,
        fill=folder_color + (255,)
    )
    
    # Main folder body
    draw.rounded_rectangle(
        [folder_x, folder_y + tab_height - 5, folder_x + folder_width, folder_y + folder_height],
        radius=12,
        fill=folder_color + (255,)
    )
    
    # Add "Applications" text (using simple drawing since we don't have a font)
    text_y = folder_y + folder_height + 30
    text_color = hex_to_rgb(COLORS['text']) + (255,)
    
    # Draw text background pill
    text_padding = 20
    draw.rounded_rectangle(
        [folder_x - text_padding, text_y - 10, folder_x + folder_width + text_padding, text_y + 30],
        radius=15,
        fill=hex_to_rgb(COLORS['surface0']) + (200,)
    )
    
    # Draw "Applications" label
    label_padding_x = 40
    label_padding_y = 8
    draw.rounded_rectangle(
        [folder_x + label_padding_x, text_y - label_padding_y, 
         folder_x + folder_width - label_padding_x, text_y + 20 + label_padding_y],
        radius=10,
        fill=hex_to_rgb(COLORS['surface1']) + (230,)
    )
    
    # Add title at top
    title = "PhotonCast"
    subtitle = "Drag to Applications to install"
    
    return img

def main():
    """Generate all icon assets."""
    print("🎨 Generating PhotonCast App Icons...")
    
    # Ensure output directory exists
    os.makedirs('resources', exist_ok=True)
    os.makedirs('assets', exist_ok=True)
    
    # Generate main app icon source (1024x1024)
    print("  Creating 1024×1024 source icon...")
    source_icon = create_photon_beam_icon(1024)
    source_icon.save('assets/icon-source-1024.png', 'PNG')
    print("    ✓ Saved to assets/icon-source-1024.png")
    
    # Generate menu bar icons
    print("  Creating menu bar template icons...")
    for size in [16, 18, 32, 36]:
        mb_icon = create_menu_bar_icon(size)
        scale = "@2x" if size > 20 else "@1x"
        base_size = size if size <= 20 else size // 2
        mb_icon.save(f'resources/MenuBarIcon_{base_size}x{base_size}{scale}.png', 'PNG')
        print(f"    ✓ Menu bar icon {base_size}x{base_size}{scale}")
    
    # Also save as PDF for vector use
    mb_icon_16 = create_menu_bar_icon(16)
    mb_icon_16.save('resources/MenuBarIcon.png', 'PNG')
    print("    ✓ Saved MenuBarIcon.png")
    
    # Generate DMG background
    print("  Creating DMG background (1600×1000)...")
    dmg_bg = create_dmg_background(1600, 1000)
    dmg_bg.save('resources/dmg-background.png', 'PNG')
    print("    ✓ Saved to resources/dmg-background.png")
    
    # Also generate 1x version (800x500)
    print("  Creating DMG background @1x (800×500)...")
    dmg_bg_1x = create_dmg_background(800, 500)
    dmg_bg_1x.save('resources/dmg-background@1x.png', 'PNG')
    print("    ✓ Saved to resources/dmg-background@1x.png")
    
    print("\n✅ Icon generation complete!")
    print("   Source: assets/icon-source-1024.png")
    print("   Menu Bar: resources/MenuBarIcon*.png")
    print("   DMG BG: resources/dmg-background.png")

if __name__ == '__main__':
    main()
