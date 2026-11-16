"""OpenAI Privacy Filter (Apache-2.0, ONNX q4, revision 7ffa9a04) as an Alias detector, with onnxruntime and tokenizers only.

Decoding follows the model card: per-token logits over 33 BIOES classes, then a constrained Viterbi path (only legal BIOES
transitions; the published default operating point has every transition bias at 0). Spans are returned in character offsets.
"""
from __future__ import annotations

from pathlib import Path

import numpy as np
import onnxruntime as ort
from tokenizers import Tokenizer

DIR = Path(__file__).resolve().parents[2] / "artifacts/alias-models/openai-privacy-filter"
CATS = ["account_number", "private_address", "private_date", "private_email", "private_person", "private_phone", "private_url",
        "secret"]
TAGS = ["O"] + [f"{b}-{c}" for c in CATS for b in "BIES"]
LABEL = {"account_number": "ACCOUNT", "private_address": "ADDRESS", "private_date": "DATE", "private_email": "EMAIL",
         "private_person": "PERSON", "private_phone": "PHONE", "private_url": "URL", "secret": "SECRET"}


def allowed(a: str, b: str) -> bool:
    if a == "O" or a[0] in "ES":
        return b == "O" or b[0] in "BS"
    return b[0] in "IE" and b[2:] == a[2:]  # inside a span: continue or close the same category


TRANS = np.array([[0.0 if allowed(a, b) else -np.inf for b in TAGS] for a in TAGS])
START = np.array([0.0 if t == "O" or t[0] in "BS" else -np.inf for t in TAGS])
END = np.array([0.0 if t == "O" or t[0] in "ES" else -np.inf for t in TAGS])


def viterbi(logp: np.ndarray) -> list[int]:
    score, back = START + logp[0], []
    for row in logp[1:]:
        cand = score[:, None] + TRANS
        back.append(cand.argmax(0))
        score = cand.max(0) + row
    path = [int((score + END).argmax())]
    for b in reversed(back):
        path.append(int(b[path[-1]]))
    return path[::-1]


class PrivacyFilter:
    def __init__(self, threads: int = 4) -> None:
        opts = ort.SessionOptions()
        opts.intra_op_num_threads = threads
        self.session = ort.InferenceSession(str(DIR / "onnx/model_q4.onnx"), opts, providers=["CPUExecutionProvider"])
        self.tok = Tokenizer.from_file(str(DIR / "tokenizer.json"))

    def detect(self, text: str) -> list[dict]:
        enc = self.tok.encode(text)
        ids = np.array([enc.ids], dtype=np.int64)
        logits = self.session.run(None, {"input_ids": ids, "attention_mask": np.ones_like(ids)})[0][0]
        logp = logits - np.log(np.exp(logits - logits.max(1, keepdims=True)).sum(1, keepdims=True)) - logits.max(1, keepdims=True)
        spans, cur = [], None
        for (s, e), tag in zip(enc.offsets, (TAGS[i] for i in viterbi(logp))):
            if tag == "O":
                cur = None
                continue
            if tag[0] in "BS" or cur is None:
                cur = {"start": s, "end": e, "label": LABEL[tag[2:]]}
                spans.append(cur)
            else:
                cur["end"] = e
            if tag[0] in "ES":
                cur = None
        for sp in spans:  # byte-level BPE offsets carry the leading space of a word
            while sp["start"] < sp["end"] and text[sp["start"]].isspace():
                sp["start"] += 1
        return spans


if __name__ == "__main__":
    pf = PrivacyFilter()
    t = "Bonjour Camille, merci de rappeler Élodie Lefèvre au 06 12 34 56 78, 12 rue des Lilas, 29200 Brest."
    for sp in pf.detect(t):
        print(sp["label"], repr(t[sp["start"]:sp["end"]]))
