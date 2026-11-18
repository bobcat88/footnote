"""Trusted host registration for one controlled Python unittest suite.

This executes repository code. Use only a trusted disposable workspace here;
production containment and process-tree ownership belong to the runtime host.
"""

import hashlib
import subprocess
import sys
import tempfile
from pathlib import Path

from .contracts import encoded, file_bytes, strict_json, within


LAUNCHER = """import importlib.util, json, sys, unittest
from pathlib import Path
sys.pycache_prefix = str(Path(sys.argv[3]).parent / 'fresh-pycache')
sys.dont_write_bytecode = True
sys.path.insert(0, sys.argv[1])
spec = importlib.util.spec_from_file_location('registered_tests', sys.argv[2])
module = importlib.util.module_from_spec(spec)
exec(compile(Path(sys.argv[2]).read_bytes(), sys.argv[2], 'exec'), module.__dict__)
suite = unittest.defaultTestLoader.loadTestsFromModule(module)
if suite.countTestCases() == 0:
    raise SystemExit('Registered suite contains no tests')
result = unittest.TextTestRunner(verbosity=2).run(suite)
Path(sys.argv[3]).write_text(json.dumps({'completed': True, 'tests_run': result.testsRun, 'passed': result.wasSuccessful()}), encoding='utf-8')
raise SystemExit(0 if result.wasSuccessful() else 1)
"""


class PythonUnittest:
    def __init__(self, *, source_directory, tests_path, tests_sha256, timeout=30):
        self.source_directory = source_directory
        self.tests_path = tests_path
        self.tests_sha256 = tests_sha256
        if type(timeout) is not int or not 1 <= timeout <= 300:
            raise ValueError("invalid test timeout")
        self.timeout = timeout
        configuration = {"launcher": LAUNCHER, "source_directory": source_directory, "tests_path": tests_path, "tests_sha256": tests_sha256, "timeout": timeout, "python": sys.version, "executable": sys.executable, "adapter_sha256": hashlib.sha256(file_bytes(Path(__file__))).hexdigest()}
        self.descriptor = {"id": "python-unittest", "version": "1", "sha256": hashlib.sha256(encoded(configuration).encode()).hexdigest()}

    def run(self, request):
        root = Path(request["workspace"])
        source = within(root, self.source_directory)
        tests = within(root, self.tests_path)
        if hashlib.sha256(file_bytes(tests)).hexdigest() != self.tests_sha256:
            raise ValueError("registered tests changed")
        # A file bounds memory use even if a test is noisy. The host must bound disk.
        with tempfile.TemporaryDirectory(prefix="footnote-verifier-") as receipt_directory, tempfile.TemporaryFile() as transcript:
            receipt_path = Path(receipt_directory) / "completion.json"
            completed = subprocess.run(["rtk", "proxy", sys.executable, "-I", "-c", LAUNCHER, str(source), str(tests), str(receipt_path)], cwd=root, stdout=transcript, stderr=subprocess.STDOUT, timeout=self.timeout, check=False)
            transcript.seek(0)
            raw = transcript.read(65_537)
            receipt = strict_json(file_bytes(receipt_path)) if receipt_path.exists() else None
        suite_passed = isinstance(receipt, dict) and receipt.get("completed") is True and receipt.get("passed") is True and type(receipt.get("tests_run")) is int and receipt["tests_run"] > 0
        return {"verification_id": request["verification_id"], "request_sha256": hashlib.sha256(encoded(request).encode()).hexdigest(), "passed": completed.returncode == 0 and suite_passed and len(raw) <= 65_536, "details": {"exit_code": completed.returncode, "suite_completion": receipt, "transcript": raw[:65_536].decode("utf-8", errors="replace"), "transcript_truncated": len(raw) > 65_536, "command": "rtk proxy <registered-python> -I -c <frozen-launcher> <owned-source> <frozen-tests> <host-completion-receipt>"}}
