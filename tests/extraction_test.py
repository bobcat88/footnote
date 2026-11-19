from pathlib import Path
import json
import subprocess
import tempfile
import os
import unittest
from PIL import Image, ImageDraw, ImageFont

REPO = Path(__file__).resolve().parents[1]
class ExtractionTests(unittest.TestCase):
    def test_real_image_and_scanned_pdf_ocr_with_original_hash_preservation(self):
        import hashlib
        with tempfile.TemporaryDirectory(dir=os.environ['TMPDIR']) as directory:
            root=Path(directory)
            image=Image.new('RGB',(1600,260),'white')
            draw=ImageDraw.Draw(image)
            font=ImageFont.truetype('C:/Windows/Fonts/arial.ttf',60)
            draw.text((40,70),'CHECKPOINT LATENCY BATCHING',fill='black',font=font)
            png=root/'image.png'; image.save(png)
            # Image-only PDF page written by Pillow, so the fixture needs no
            # PDF-authoring library (PyMuPDF was AGPL-3.0 and is not a dependency).
            pdf=root/'scan.pdf'; image.convert('RGB').save(pdf,'PDF',resolution=200.0)
            binary=REPO/'target/release/footnote.exe'
            db=root/'k.db'
            original={p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [png,pdf]}
            def call(*args):
                p=subprocess.run([str(binary),'--db',str(db),*args],capture_output=True,text=True,check=True)
                return json.loads(p.stdout)
            v=call('index','--root',str(root),'--allow',str(root))
            self.assertEqual(v['indexed'],2); self.assertEqual(v['failed'],0)
            result=call('search','checkpoint')['results']
            self.assertEqual(len(result),2)
            self.assertEqual({Path(h['path']).suffix for h in result},{'.pdf','.png'})
            for h in result:
                self.assertEqual(h['page'],1);self.assertEqual(h['freshness'],'current')
                # Lean search (#20) leaves the extractor out of hits; `get` still has it.
                extractor=call('get',str(h['id']))['extractor']
                self.assertIn('rapidocr',extractor)
                # Licence regression lock: extraction must stay on PDFium, not PyMuPDF.
                self.assertIn('pdfium',extractor)
                self.assertNotIn('pymupdf',extractor)
            self.assertEqual(original,{p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in [png,pdf]})
            self.assertEqual(call('index','--root',str(root),'--allow',str(root))['unchanged'],2)
if __name__=='__main__':unittest.main()
