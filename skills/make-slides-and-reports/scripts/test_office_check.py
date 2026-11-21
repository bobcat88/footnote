import os
import sys
import tempfile
import unittest
import zipfile

sys.path.insert(0, os.path.dirname(__file__))
from office_check import check  # noqa: E402

SLIDE = ('<p:sld xmlns:p="p" xmlns:a="a" show="0"><a:t>{text}</a:t>'
         '<p:pic><p:nvPicPr><p:cNvPr id="4" name="Picture 3"/></p:nvPicPr></p:pic></p:sld>')


def make(parts):
    fd, path = tempfile.mkstemp(suffix=".pptx")
    os.close(fd)
    with zipfile.ZipFile(path, "w") as z:
        for name, body in parts.items():
            z.writestr(name, body)
    return path


class OfficeCheck(unittest.TestCase):
    def ids(self, parts):
        path = make(parts)
        try:
            return sorted(f["id"] for f in check(path))
        finally:
            os.remove(path)

    def test_clean_deck(self):
        ok = '<p:sld xmlns:p="p" xmlns:a="a"><a:t>Revenue grew 4 %</a:t><p:pic><p:cNvPr id="4" name="x" descr="Revenue chart"/></p:pic></p:sld>'
        self.assertEqual(self.ids({"ppt/slides/slide1.xml": ok}), [])

    def test_leaks_are_found(self):
        parts = {
            "ppt/slides/slide1.xml": SLIDE.format(text="TODO see C:\\Users\\me\\deck.pptx, synthÃ¨se"),
            "ppt/comments/comment1.xml": "<p:cmLst/>",
            "ppt/notesSlides/notesSlide1.xml": "<p:notes/>",
            "docProps/core.xml": "<cp:coreProperties><dc:creator>A Person</dc:creator></cp:coreProperties>",
        }
        self.assertEqual(self.ids(parts), ["O1-placeholder", "O2-local-path", "O3-mojibake", "O4-comments",
                                           "O5-hidden-slide", "O6-notes", "O7-metadata", "O8-alt-text"])


if __name__ == "__main__":
    unittest.main()
