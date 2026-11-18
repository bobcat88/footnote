import copy
import json
import tempfile
import unittest
from pathlib import Path

from integration.footnote import prepare, sha, validate_context


class ContextTests(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name).resolve()
        self.workspace = self.root / "work"
        self.workspace.mkdir()
        self.source = self.workspace / "module.py"
        self.source.write_text("def f(): return 1")
        self.doc = self.root / "contract.txt"
        self.doc.write_text("f must return 1")
        outer = self
        class Client:
            sandbox = outer.root
            state = outer.root / "state"
            binary_sha256 = "a" * 64
            code = {"root": str(outer.workspace), "stale": [], "symbols": [{"path": "module.py"}], "related": {"memory": ["not trusted implicitly"]}}
            memory = {"memories": [{"scope": "project", "text": "decision"}]}
            docs = {"results": [{"path": str(outer.doc), "freshness": "current", "text": "f must return 1"}]}
            def call(self, operation, *args, **kwargs):
                return copy.deepcopy({"code": self.code, "memory-recall": self.memory, "search": self.docs}[operation])
        self.client = Client()

    def prepare(self, **kwargs):
        return prepare(self.client, project="project", workspace=self.workspace, symbol="f", query="f", documents=[self.doc], **kwargs)

    def test_actual_sources_bound_and_unfiltered_related_not_forwarded(self):
        receipt = self.prepare()
        self.assertEqual(receipt["sources"][str(self.source)], sha(self.source))
        self.assertNotIn("related", receipt["code"])
        self.assertEqual(receipt["context_bytes"], len(json.dumps(receipt, ensure_ascii=False, indent=2).encode()))
        validate_context(receipt)

    def test_source_change_requires_reprepare(self):
        receipt = self.prepare()
        self.source.write_text("def f(): return 2")
        with self.assertRaises(ValueError): validate_context(receipt)

    def test_stale_or_wrong_project_graph_refused(self):
        self.client.code["stale"] = ["module.py"]
        with self.assertRaises(ValueError): self.prepare()
        self.client.code["stale"] = []
        self.client.code["root"] = str(self.root / "foreign")
        with self.assertRaises(ValueError): self.prepare()

    def test_foreign_memory_and_stale_docs_refused(self):
        self.client.memory["memories"][0]["scope"] = "foreign"
        with self.assertRaises(ValueError): self.prepare()
        self.client.memory["memories"][0]["scope"] = "project"
        self.client.docs["results"][0]["freshness"] = "stale"
        with self.assertRaises(ValueError): self.prepare()

    def test_unlisted_docs_and_oversize_context_refused(self):
        with self.assertRaises(ValueError): self.prepare(limit_bytes=10)
        self.client.docs["results"][0]["path"] = str(self.root / "other.txt")
        with self.assertRaises(ValueError): self.prepare()


if __name__ == "__main__":
    unittest.main()
