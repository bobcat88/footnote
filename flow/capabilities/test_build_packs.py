import hashlib
import json
import tempfile
import unittest
from pathlib import Path

import yaml

from build_packs import AI_ROOT, PROJECT, build_pack, normalize_skill


# The authored cards stay in the notes vault; set FLOW_SKILL_SOURCES to its AI-Agents folder to build from them.
SOURCES = unittest.skipUnless((AI_ROOT / "skills/python-performance/SKILL.md").is_file(), "authored skill cards not available")


class PackTests(unittest.TestCase):
    def test_shipped_pack_matches_its_manifest(self):
        root = PROJECT.parent / "skills"
        manifest = json.loads((root / "pack-manifest.json").read_text(encoding="utf-8"))
        for relative, expected_hash in manifest["files"].items():
            self.assertEqual(hashlib.sha256((root / relative).read_bytes()).hexdigest(), expected_hash, relative)
        self.assertEqual(sorted(p.name for p in (root / "skills").iterdir()), sorted(manifest["skills"]))
        for name in manifest["skills"]:
            card = normalize_skill((root / "skills" / name / "SKILL.md").read_text(encoding="utf-8"), name)
            self.assertTrue(yaml.safe_load(card.split("---", 2)[1])["description"].strip())

    @SOURCES
    def test_real_all_pack_has_valid_standalone_skills_and_hashes(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "all"
            receipt = build_pack("all", output)
            self.assertEqual(len(receipt["skills"]), 18)  # 17 new cards + existing website dependency
            self.assertFalse(receipt["installed"])
            self.assertTrue(receipt["external_dependencies"])
            for relative, expected_hash in receipt["files"].items():
                self.assertEqual(hashlib.sha256((output / relative).read_bytes()).hexdigest(), expected_hash)
            for name in receipt["skills"]:
                text = (output / "skills" / name / "SKILL.md").read_text(encoding="utf-8")
                card = yaml.safe_load(text.split("---", 2)[1])
                self.assertEqual(card["name"], name)
                self.assertNotIn("tags", card)
                self.assertIn("experimental", card["metadata"]["distribution-status"])
            self.assertTrue((output / "skills/rust-performance/scripts/compare_receipts.py").is_file())
            self.assertTrue((output / "_shared/qloop-ui-protocol.md").is_file())
            self.assertTrue((output / "skills/website-launch-checklist/references/launch-checks.md").is_file())
            for path in output.rglob("*.md"):
                self.assertNotIn("[[", path.read_text(encoding="utf-8"))

    @SOURCES
    def test_non_ui_pack_omits_ui_protocol_and_preserves_selection(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "systems"
            receipt = build_pack("systems", output)
            self.assertEqual(set(receipt["skills"]), {"ship-rust-tool", "rust-performance", "sqlite-sql-performance", "ci-release-engineering"})
            self.assertFalse(receipt["external_dependencies"])
            self.assertFalse((output / "_shared").exists())

    @SOURCES
    def test_every_named_pack_can_be_built(self):
        manifest = json.loads((PROJECT / "capabilities/packs.json").read_text())
        with tempfile.TemporaryDirectory() as temp:
            for pack in manifest["packs"]:
                with self.subTest(pack=pack):
                    build_pack(pack, Path(temp) / pack)

    def test_existing_output_is_never_overwritten(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "existing"
            output.mkdir()
            sentinel = output / "user.txt"
            sentinel.write_text("preserve")
            with self.assertRaises(ValueError):
                build_pack("systems", output)
            self.assertEqual(sentinel.read_text(), "preserve")
            self.assertEqual(list(output.iterdir()), [sentinel])

    def test_invalid_pack_and_protected_destination_write_nothing(self):
        with tempfile.TemporaryDirectory() as temp:
            output = Path(temp) / "missing"
            with self.assertRaises(ValueError):
                build_pack("unknown", output)
            self.assertFalse(output.exists())
        with self.assertRaises(ValueError):
            build_pack("systems", AI_ROOT / "skills" / "must-not-export")
        with self.assertRaises(ValueError):
            build_pack("systems", PROJECT / "capabilities" / "must-not-export")
        with tempfile.TemporaryDirectory() as temp:
            for runtime in (".codex", ".claude", ".agents", ".gemini", ".dsh"):
                output = Path(temp) / runtime / "skills" / "must-not-export"
                with self.subTest(runtime=runtime), self.assertRaises(ValueError):
                    build_pack("systems", output)
                self.assertFalse(output.exists())

    def test_invalid_names_and_missing_dependencies_fail_before_writing(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp)
            project = root / "project"
            capabilities = project / "capabilities"
            capabilities.mkdir(parents=True)
            manifest = {"schema_version": 1, "packs": {"bad": ["../escape"]}, "measurement_skills": [], "ui_skills": [], "external_ui_dependencies": []}
            (capabilities / "packs.json").write_text(json.dumps(manifest))
            output = root / "output"
            with self.assertRaises(ValueError):
                build_pack("bad", output, ai_root=root / "ai", project=project)
            self.assertFalse(output.exists())
            manifest["packs"]["bad"] = ["example"]
            (capabilities / "packs.json").write_text(json.dumps(manifest))
            folder = root / "ai/skills/example"
            folder.mkdir(parents=True)
            (folder / "SKILL.md").write_text("---\nname: example\ndescription: Example workflow.\n---\n[missing](../missing/SKILL.md)\n")
            with self.assertRaises(ValueError):
                build_pack("bad", output, ai_root=root / "ai", project=project)
            self.assertFalse(output.exists())

    def test_normalization_preserves_provenance_and_removes_vault_only_fields(self):
        source = "---\nname: example\ndescription: Example task.\ntype: reference\ntags: [domain/ai/agents]\nmetadata:\n  version: 1\n---\n# Example\nUp: [[Parent]]\n#domain/ai/agents\nUse [[Some Note|its label]].\n"
        result = normalize_skill(source, "example")
        card = yaml.safe_load(result.split("---", 2)[1])
        self.assertEqual(card["metadata"]["version"], "1")
        self.assertEqual(card["metadata"]["source-sha256"], hashlib.sha256(source.encode()).hexdigest())
        self.assertIn("Use its label.", result)
        self.assertNotIn("[[", result)
        with self.assertRaises(ValueError):
            normalize_skill(source, "different")


if __name__ == "__main__":
    unittest.main()
