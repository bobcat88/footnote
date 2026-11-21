"""Local extraction protocol v1. No networking; models ship in the OCR wheel.
Input is a Rust-owned snapshot, never the live original. stdout is bounded JSON.

PDF text extraction and page rasterization use PDFium through pypdfium2
(Apache-2.0 / BSD-3-Clause). PyMuPDF (AGPL-3.0) is deliberately not imported:
it made every distribution of this tool copyleft.
"""
import importlib.metadata
import json
from pathlib import Path
import sys

MAX_CHARS = 1_000_000
MAX_PAGES = 100
MAX_PIXELS = 20_000_000
SAFE_IMAGE_FORMATS = ('PNG', 'JPEG', 'WEBP', 'BMP', 'GIF', 'TIFF')

def extract(path):
    import pypdfium2 as pdfium
    import pypdfium2.raw as pdfium_c
    from PIL import Image
    import numpy as np
    Image.MAX_IMAGE_PIXELS = MAX_PIXELS
    engine = None
    ocr_used = False
    def ocr(image):
        nonlocal engine, ocr_used
        if image.width * image.height > MAX_PIXELS:
            raise ValueError('image pixel limit exceeded')
        if engine is None:
            from rapidocr_onnxruntime import RapidOCR
            engine = RapidOCR(intra_op_num_threads=1, inter_op_num_threads=1)
        ocr_used = True
        result, _ = engine(np.asarray(image.convert('RGB')))
        return '\n'.join(row[1] for row in result or [] if row[2] >= 0.5)
    pages = []
    if path.suffix.lower() == '.pdf':
        try:
            doc = pdfium.PdfDocument(path)
        except pdfium.PdfiumError as exc:
            # PDFium refuses to load a document that needs a password.
            raise ValueError('encrypted PDF or unreadable PDF') from exc
        try:
            if len(doc) > MAX_PAGES:
                raise ValueError('encrypted PDF or page limit exceeded')
            for n in range(len(doc)):
                page = doc[n]
                try:
                    textpage = page.get_textpage()
                    try:
                        # PDFium emits CRLF; normalise so chunk offsets and
                        # character budgets match the rest of the pipeline.
                        text = textpage.get_text_range().replace('\r\n', '\n').replace('\r', '\n')
                    finally:
                        textpage.close()
                    # OCR full raster for scans and mixed pages containing images.
                    has_images = any(
                        obj.type == pdfium_c.FPDF_PAGEOBJ_IMAGE for obj in page.get_objects()
                    )
                    if not text.strip() or has_images:
                        width, height = page.get_size()
                        scale = min(2.0, (MAX_PIXELS / max(width * height, 1)) ** 0.5 * 0.9)
                        bitmap = page.render(scale=scale)
                        try:
                            extra = ocr(bitmap.to_pil())
                        finally:
                            bitmap.close()
                        # Keep layer text and OCR as separately marked evidence.
                        if extra.strip():
                            text += '\n[OCR transcription, may contain errors]\n' + extra
                    pages.append({'page': n + 1, 'text': text})
                    if sum(len(p['text']) for p in pages) > MAX_CHARS:
                        raise ValueError('extracted text limit exceeded')
                finally:
                    page.close()
        finally:
            doc.close()
    else:
        with Image.open(path, formats=SAFE_IMAGE_FORMATS) as im:
            if im.width * im.height > MAX_PIXELS:
                raise ValueError('image pixel limit exceeded')
            pages = [{'page': 1, 'text': ocr(im)}]
    versions = {n: importlib.metadata.version(n) for n in ['pypdfium2', 'rapidocr-onnxruntime', 'onnxruntime']}
    return {'status': 'indexed' if any(p['text'].strip() for p in pages) else 'empty',
            'extractor': 'pdfium+rapidocr-v1:' + json.dumps(versions, sort_keys=True) + ';ocr_used=' + str(ocr_used) + ';empty_pages=' + json.dumps([p['page'] for p in pages if not p['text'].strip()]),
            'ocr_used': ocr_used, 'pages': pages}

if __name__ == '__main__':
    try:
        value = extract(Path(sys.argv[1]))
    except Exception as exc:
        value = {'status': 'failed', 'error': type(exc).__name__, 'extractor': 'local-extractor-v1', 'pages': []}
    print(json.dumps(value, ensure_ascii=True))
