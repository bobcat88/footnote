"""Rebuild site/src/fonts/*.woff2 from the Fontsource variable fonts (SIL OFL).
Usage: python site/scripts/subset-fonts.py <dir containing node_modules/@fontsource-variable> <out dir>
Needs: pip install fonttools brotli. Narrows the weight axis to what the page uses and keeps only the glyphs it shows."""
import sys, os, io
from fontTools.ttLib import TTFont
from fontTools.varLib import instancer
from fontTools import subset

src, out = sys.argv[1], sys.argv[2]
UNI = list(range(0x20, 0x7F)) + list(range(0xA0, 0x100)) + [0x2013, 0x2014, 0x2018, 0x2019, 0x201C, 0x201D, 0x2022, 0x2026, 0x2192, 0x2193, 0x2197, 0x2212]
FONTS = {  # name: (package, file, wght range)
    "plus-jakarta-sans": ("plus-jakarta-sans", "plus-jakarta-sans-latin-wght-normal.woff2", (300, 700)),
    "hanken-grotesk": ("hanken-grotesk", "hanken-grotesk-latin-wght-normal.woff2", (400, 600)),
    "jetbrains-mono": ("jetbrains-mono", "jetbrains-mono-latin-wght-normal.woff2", (400, 600)),
}
os.makedirs(out, exist_ok=True)
for name, (pkg, fn, (lo, hi)) in FONTS.items():
    f = TTFont(os.path.join(src, "node_modules/@fontsource-variable", pkg, "files", fn))
    f = instancer.instantiateVariableFont(f, {"wght": (lo, hi)})
    b = io.BytesIO(); f.flavor = None; f.save(b); b.seek(0); f = TTFont(b, lazy=False)  # reload: the instanced font is lazy and trips the subsetter
    o = subset.Options(); o.flavor = "woff2"; o.layout_features = ["kern", "tnum", "lnum", "case"]; o.notdef_outline = True
    o.name_IDs = [1, 2, 3, 4, 6]; o.hinting = False; o.desubroutinize = True
    s = subset.Subsetter(o); s.populate(unicodes=UNI); s.subset(f)
    f.flavor = "woff2"; p = os.path.join(out, name + ".woff2"); f.save(p); print(p, os.path.getsize(p))
