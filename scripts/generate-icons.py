#!/usr/bin/env python3
"""Generate all app icons from SVG sources.

Requires: pip install cairosvg Pillow
"""

import os
import subprocess
import sys
from pathlib import Path

try:
    import cairosvg
    from PIL import Image
except ImportError:
    print("Missing dependencies. Install with:")
    print("  pip install cairosvg Pillow")
    sys.exit(1)

ROOT = Path(__file__).resolve().parent.parent
ICONS_DIR = ROOT / "src-tauri" / "icons"
APP_SVG = ROOT / "icon.svg"
TRAY_SVG = ROOT / "tray-icon.svg"

SIZES = [32, 128, 256, 512, 1024]


def svg_to_png(svg_path: Path, png_path: Path, size: int):
    cairosvg.svg2png(
        url=str(svg_path),
        write_to=str(png_path),
        output_width=size,
        output_height=size,
    )
    # Ensure RGBA 8-bit (Tauri requires this)
    img = Image.open(png_path).convert("RGBA")
    img.save(png_path)


def generate_app_icons():
    print("Generating app icons from", APP_SVG.name)
    for size in SIZES:
        out = ICONS_DIR / f"{size}x{size}.png"
        svg_to_png(APP_SVG, out, size)
        print(f"  {out.name}")

    # 128x128@2x is 256x256
    src = ICONS_DIR / "256x256.png"
    dst = ICONS_DIR / "128x128@2x.png"
    dst.write_bytes(src.read_bytes())
    print(f"  {dst.name}")


def generate_tray_icon():
    print("Generating tray icon from", TRAY_SVG.name)
    for suffix, size in [("tray-icon.png", 22), ("tray-icon@2x.png", 44)]:
        out = ICONS_DIR / suffix
        svg_to_png(TRAY_SVG, out, size)
        print(f"  {out.name}")


def generate_icns():
    print("Generating icon.icns")
    iconset = ICONS_DIR / "icon.iconset"
    iconset.mkdir(exist_ok=True)

    mapping = {
        "icon_32x32.png": "32x32.png",
        "icon_128x128.png": "128x128.png",
        "icon_128x128@2x.png": "256x256.png",
        "icon_256x256.png": "256x256.png",
        "icon_256x256@2x.png": "512x512.png",
        "icon_512x512.png": "512x512.png",
        "icon_512x512@2x.png": "1024x1024.png",
    }

    for dst_name, src_name in mapping.items():
        (iconset / dst_name).write_bytes((ICONS_DIR / src_name).read_bytes())

    subprocess.run(
        ["iconutil", "-c", "icns", str(iconset), "-o", str(ICONS_DIR / "icon.icns")],
        check=True,
    )

    # Cleanup
    import shutil
    shutil.rmtree(iconset)
    print(f"  icon.icns")


def generate_ico():
    print("Generating icon.ico")
    imgs = []
    for size in [32, 128, 256]:
        img = Image.open(ICONS_DIR / f"{size}x{size}.png")
        imgs.append(img)
    imgs[0].save(
        ICONS_DIR / "icon.ico",
        format="ICO",
        sizes=[(img.width, img.height) for img in imgs],
        append_images=imgs[1:],
    )
    print(f"  icon.ico")


def main():
    ICONS_DIR.mkdir(parents=True, exist_ok=True)

    generate_app_icons()
    generate_tray_icon()
    generate_ico()

    if sys.platform == "darwin":
        generate_icns()
    else:
        print("Skipping .icns (not on macOS)")

    print("Done.")


if __name__ == "__main__":
    main()
