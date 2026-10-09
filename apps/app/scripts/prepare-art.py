"""Regenerate prefiltered PNGs from unchanged source artwork (development-only Pillow)."""
from pathlib import Path
from PIL import Image

ART = Path(__file__).resolve().parents[1] / "assets" / "art"
OUT = ART / "sizes"
OUT.mkdir(exist_ok=True)

# Pillow filters RGBA in premultiplied alpha to avoid dark/bright matte edges.
# Originals have no ICC/gamma profile; do not introduce a new color space.
for source in sorted(ART.glob("*.png")):
    with Image.open(source) as original:
        if original.info.get("icc_profile") or original.info.get("gamma"):
            raise ValueError(f"Review color profile before converting {source.name}")
        widths = (64, 96, 128, 192, 256, 384, 512) if source.stem.startswith("frame-") else (32, 48, 64, 128, 256, 384, 512, 768)
        for width in widths:
            size = (width, round(original.height * width / original.width))
            derivative = original.resize(size, Image.Resampling.LANCZOS)
            # Strip generated-image provenance blobs only from derivatives; originals stay intact.
            derivative.info.clear()
            derivative.save(OUT / f"{source.stem}-{width}.png", optimize=True)
