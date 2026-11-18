"""Explicit test adapters, not the real Footnote token layer or model runtime."""

import uuid

from .contracts import encoded
from .governor import digest


class FixtureTokenLayer:
    mode = "fixture"

    def activate(self, brief):
        return {
            "active": True,
            "mode": "fixture",
            "task_id": brief["task_id"],
            "attempt_id": brief["attempt_id"],
            "brief_sha256": digest(encoded(brief).encode()),
            "adapter": "fixture-token-layer-not-footnote",
            "version": "1",
            "activation_id": str(uuid.uuid4()),
            "predicted_input_tokens": None,
            "predicted_output_tokens": None,
        }


def fixture_report(brief, *, artifacts, summary="Fixture work completed", memories=None):
    return {
        "task_id": brief["task_id"],
        "attempt_id": brief["attempt_id"],
        "summary": summary,
        "artifacts": artifacts,
        "unresolved": [],
        "usage": {"source": "unknown", "input_tokens": None, "cached_input_tokens": None, "output_tokens": None},
        "candidate_memories": memories or [],
        "runtime_turn_id": "fixture:" + brief["attempt_id"],
    }
