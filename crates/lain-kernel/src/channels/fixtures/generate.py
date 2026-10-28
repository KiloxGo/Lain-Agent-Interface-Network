"""Regenerate synthetic decoder fixtures; requires Pillow with animated WebP.

These fixtures contain generated colors only. Pillow 11.2.1 generated the initial
files; Pillow is not a Rust build/test/runtime dependency.
"""
from pathlib import Path
from PIL import Image

root = Path(__file__).resolve().parent
colors = [(255, 0, 0) if n == 0 else (0, 255, 0) if n == 8 else (0, 0, 255)
          if n == 11 else (n * 17 % 256, n * 23 % 256, n * 29 % 256) for n in range(70)]
frames = [Image.new("RGBA", (4, 4), color + (255,)) for color in colors]
# APNG poster is not part of its animation: the first animation frame is red.
Image.new("RGBA", (4, 4), (80, 80, 80, 255)).save(
    root / "poster-animation.png", save_all=True, default_image=True,
    append_images=frames[:12], duration=[100] * 12, loop=0, disposal=0, blend=0)
frames[0].save(root / "animation.webp", save_all=True, append_images=frames[1:12],
               duration=100, loop=0, lossless=True, minimize_size=False)
frames[0].save(root / "overlong.gif", save_all=True, append_images=frames[1:],
               duration=100, loop=0, optimize=False)
print("Generated APNG (poster + 12 frames), WebP (12 frames), GIF (70 frames)")
