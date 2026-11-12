"""Offline CPU ONNX embeddings. Two entry points, one recipe.

    embed.py MODEL_DIR REQUEST.json        one request from a bounded local file, one JSON answer on stdout (legacy, one process per call)
    embed.py --serve MODEL_DIR             resident worker: verifies the model once, then answers one JSON line per JSON line on stdin

The worker exists because loading the model costs about a second and encoding one query costs a few milliseconds. The hash check and
the recipe check happen before the first answer; a request never selects a different model, a path or a recipe.
"""
import hashlib
import json
import sys
from pathlib import Path

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer

MAX_BATCH = 256
MAX_TEXT_BYTES = 64000


class Embedder:
    def __init__(self, model_path):
        model = Path(model_path)
        manifest_bytes = (model / "manifest.json").read_bytes()
        m = json.loads(manifest_bytes)
        if m["pooling"] != "attention-mask-mean-l2" or not 1 <= m["max_length"] <= 512 or m["dimension"] != 384:
            raise ValueError("unsupported embedding recipe")
        for name in ["model.onnx", "tokenizer.json"]:
            path = model / name
            if path.is_symlink() or hashlib.sha256(path.read_bytes()).hexdigest() != m["files"][name]["sha256"]:
                raise ValueError("model artifact hash mismatch: " + name)
        self.m = m
        self.model_sha256 = hashlib.sha256(manifest_bytes).hexdigest()
        self.tokenizer = Tokenizer.from_file(str(model / "tokenizer.json"))
        self.tokenizer.enable_truncation(max_length=m["max_length"])
        pad_token = m.get("pad_token", "<pad>")
        pad_id = self.tokenizer.token_to_id(pad_token)
        if pad_id is None:
            raise ValueError("pad token not in vocabulary: " + pad_token)
        self.tokenizer.enable_padding(pad_id=pad_id, pad_token=pad_token)
        options = ort.SessionOptions()
        options.intra_op_num_threads = 2
        options.inter_op_num_threads = 1
        self.session = ort.InferenceSession(str(model / "model.onnx"), sess_options=options, providers=["CPUExecutionProvider"])

    def encode(self, request):
        texts, kind = request["texts"], request["kind"]
        if kind not in ("query", "passage"):
            raise ValueError("kind must be query or passage")
        if not 1 <= len(texts) <= 10000 or any(not isinstance(t, str) or len(t.encode()) > MAX_TEXT_BYTES for t in texts):
            raise ValueError("embedding input bound exceeded")
        m = self.m
        prefix = m["query_prefix"] if kind == "query" else m["passage_prefix"]
        result = []
        truncated = 0
        for start in range(0, len(texts), 8):
            tokens = self.tokenizer.encode_batch([prefix + t for t in texts[start:start + 8]])
            ids = np.asarray([t.ids for t in tokens], dtype=np.int64)
            mask = np.asarray([t.attention_mask for t in tokens], dtype=np.int64)
            truncated += int((mask.sum(axis=1) >= m["max_length"]).sum())
            inputs = {"input_ids": ids, "attention_mask": mask, "token_type_ids": np.zeros_like(ids)}
            output = self.session.run(None, {i.name: inputs[i.name] for i in self.session.get_inputs()})[0]
            vectors = (output * mask[:, :, None]).sum(axis=1) / mask.sum(axis=1)[:, None]
            vectors /= np.maximum(np.linalg.norm(vectors, axis=1, keepdims=True), 1e-12)
            if vectors.shape[1] != m["dimension"] or not np.isfinite(vectors).all():
                raise ValueError("invalid embedding output")
            result.extend(vectors.tolist())
        return {"vectors": result, "model_sha256": self.model_sha256, "truncated_at_limit": truncated,
                "runtime": {"onnxruntime": ort.__version__, "numpy": np.__version__}}


def serve(model_path):
    embedder = Embedder(model_path)
    print(json.dumps({"ready": True, "model_sha256": embedder.model_sha256}), flush=True)
    for line in sys.stdin:
        try:
            if len(line) > 64_000_000:
                raise ValueError("request exceeds 64 MB")
            request = json.loads(line)
            if len(request["texts"]) > MAX_BATCH:
                raise ValueError(f"at most {MAX_BATCH} texts per request in serve mode")
            answer = embedder.encode(request)
        except Exception as error:  # one bad request must not end the worker
            answer = {"error": str(error)}
        print(json.dumps(answer), flush=True)


if __name__ == "__main__":
    try:
        if sys.argv[1] == "--serve":
            serve(sys.argv[2])
        else:
            request_path = Path(sys.argv[2])
            if request_path.stat().st_size > 64000000:
                raise ValueError("request exceeds 64 MB")
            print(json.dumps(Embedder(sys.argv[1]).encode(json.loads(request_path.read_text(encoding="utf-8")))))
    except Exception as error:
        print(json.dumps({"error": str(error)}))
        sys.exit(1)
