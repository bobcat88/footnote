"""User-authorized measurement-only activation for the Flow benchmark.

This is not the deferred predictor. Host code supplies authoritative usage;
activation alone never claims that any usage was captured.
"""

import hashlib
import uuid

from orchestration.contracts import encoded, text


DECISION = "Evaluation only: measurement adapter authorized; no token predictions"


class MeasurementOnlyAdapter:
    mode = "real"
    name = "flow-evaluation-measurement-only"
    version = "1"

    def __init__(self, run_id):
        text(run_id, "evaluation run ID")
        self.run_id = run_id
        self.activations = {}
        self.calls = {}

    def activate(self, brief):
        if brief.get("mode") != "real" or DECISION not in brief.get("project_decisions", []):
            raise ValueError("measurement-only activation requires an explicit evaluation project")
        if not brief.get("runtime", {}).get("adapter", "").startswith("flow-evaluation:"):
            raise ValueError("measurement adapter is restricted to evaluation runtimes")
        attempt = brief["attempt_id"]
        if attempt in self.activations:
            raise ValueError("duplicate activation; reconcile the existing evaluation attempt")
        result = {"active": True, "mode": "real", "task_id": brief["task_id"], "attempt_id": attempt, "brief_sha256": hashlib.sha256(encoded(brief).encode()).hexdigest(), "adapter": self.name, "version": self.version, "activation_id": str(uuid.uuid4()), "evaluation_run_id": self.run_id, "purpose": "measurement-only", "prediction_enabled": False, "predicted_input_tokens": None, "predicted_output_tokens": None}
        self.activations[attempt] = dict(result)
        return result

    def record_call(self, *, attempt_id, agent_id, call_id, usage):
        """Normalize trusted host per-call counts; never accept a cumulative total."""
        if attempt_id not in self.activations:
            raise ValueError("usage is not linked to an activated attempt")
        for value, label in ((agent_id, "agent ID"), (call_id, "call ID")):
            text(value, label)
        if call_id in self.calls:
            raise ValueError("duplicate inference-call usage")
        if not isinstance(usage, dict) or set(usage) != {"source", "accounting", "input_tokens", "cached_input_tokens", "output_tokens"} or usage["source"] != "runtime" or usage["accounting"] != "per-call":
            raise ValueError("authoritative runtime per-call usage required")
        for key in ("input_tokens", "cached_input_tokens", "output_tokens"):
            value = usage[key]
            if value is not None and (type(value) is not int or value < 0):
                raise ValueError("invalid actual usage count")
        if usage["input_tokens"] is not None and usage["cached_input_tokens"] is not None and usage["cached_input_tokens"] > usage["input_tokens"]:
            raise ValueError("cached input is a subset of input")
        call = {"agent_id": agent_id, "call_id": call_id, "attempt_id": attempt_id, **usage, "source": "real"}
        self.calls[call_id] = dict(call)
        return call

    def snapshot(self):
        return {"evaluation_run_id": self.run_id, "adapter": self.name, "version": self.version, "purpose": "measurement-only", "prediction_enabled": False, "capture_complete": False, "activations": [dict(item) for item in self.activations.values()], "calls": [dict(item) for item in self.calls.values()], "limitations": "Host-supplied runtime telemetry. This adapter cannot prove exhaustive capture, authenticate provider data, launch models or persist recovery state. Preserve receipts in the benchmark host."}
