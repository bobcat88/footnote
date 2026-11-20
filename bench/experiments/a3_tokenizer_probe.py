"""Compare GGUF server token IDs with the original tokenizer for a small fixture."""
import json
from pathlib import Path
import socket
import subprocess
import time
import urllib.request

from huggingface_hub import hf_hub_download
from tokenizers import Tokenizer

import os
ROOT = Path(__file__).resolve().parents[2]
SERVER = Path(os.environ.get("FOOTNOTE_LLAMA_SERVER", "llama-server"))
MODELS = (
    ("Octen/Octen-Embedding-4B", "Octen-Embed-4B-Q8_0.gguf"),
    ("codefuse-ai/F2LLM-v2-1.7B", "F2LLM-V2-1.7B-Q8_0.gguf"),
    ("microsoft/harrier-oss-v1-0.6b", "harrier-oss-v1-0.6b-q8_0.gguf"),
)
SAMPLES = ("Hello", "Instruct: Find the cause\nQuery: Rust bug", "Réparation du démon sous Linux.")


def request(url: str, body: dict) -> dict:
    req = urllib.request.Request(url, json.dumps(body).encode(), {"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=90) as response:
        return json.load(response)


def main() -> None:
    for repo, filename in MODELS:
        tokenizer = Tokenizer.from_file(hf_hub_download(repo, "tokenizer.json", token=False))
        with socket.socket() as sock:
            sock.bind(("127.0.0.1", 0))
            port = sock.getsockname()[1]
        url = f"http://127.0.0.1:{port}"
        proc = subprocess.Popen([str(SERVER), "-m", str(ROOT / "artifacts/gguf" / filename), "--embedding",
                                 "--pooling", "last", "-ngl", "0", "-np", "1", "-c", "1024", "--host", "127.0.0.1",
                                 "--port", str(port), "--no-webui"], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        try:
            for _ in range(180):
                try:
                    with urllib.request.urlopen(url + "/health", timeout=2):
                        break
                except OSError:
                    if proc.poll() is not None:
                        raise RuntimeError(f"Server exited {proc.returncode} for {repo}")
                    time.sleep(1)
            else:
                raise RuntimeError(f"Server startup timed out for {repo}")
            for text in SAMPLES:
                expected = tokenizer.encode(text, add_special_tokens=True).ids
                actual = request(url + "/tokenize", {"content": text, "add_special": True})["tokens"]
                if actual != expected:
                    raise RuntimeError(f"{repo} token mismatch: {text!r}: {actual} != {expected}")
            usage = request(url + "/v1/embeddings", {"input": [SAMPLES[0]]}).get("usage", {})
            print(json.dumps({"repo": repo, "fixtures": len(SAMPLES), "matching": True, "embedding_usage": usage}), flush=True)
        finally:
            proc.terminate()
            proc.wait(timeout=30)


if __name__ == "__main__":
    main()
