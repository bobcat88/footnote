"""Validate normalized per-call telemetry; never invent unavailable runtime data."""

import argparse
import json
import math
from pathlib import Path

from orchestration.contracts import strict_json


ARMS = {"baseline", "skills", "flow"}
COUNTS = ("input_tokens", "cached_input_tokens", "output_tokens")


def aggregate(calls):
    result = {"calls": len(calls)}
    for key in COUNTS:
        values = [call[key] for call in calls]
        result[key] = sum(values) if values and all(value is not None for value in values) else None
        result[key + "_observed_calls"] = sum(value is not None for value in values)
    result["total_tokens"] = result["input_tokens"] + result["output_tokens"] if result["input_tokens"] is not None and result["output_tokens"] is not None else None
    return result


def summarize(receipt):
    if not isinstance(receipt, dict) or type(receipt.get("schema_version")) is not int or receipt["schema_version"] != 1 or receipt.get("arm") not in ARMS or receipt.get("mode") not in {"real", "fixture"}:
        raise ValueError("unsupported run identity/schema")
    if receipt.get("outcome") not in {"accepted", "failed", "timeout", "uncertain"}:
        raise ValueError("explicit terminal outcome required")
    events = receipt["events_ms"]
    if not isinstance(events, dict) or events.get("submitted") != 0:
        raise ValueError("normalize same-host monotonic events to submitted=0")
    for key, value in events.items():
        if value is not None and (type(value) not in (int, float) or not math.isfinite(value) or value < 0):
            raise ValueError("invalid event time")
    terminal = events.get("terminal")
    if terminal is None or any(value is not None and value > terminal for value in events.values()):
        raise ValueError("missing or inconsistent terminal time")
    if (receipt["outcome"] == "accepted") != (events.get("accepted") is not None):
        raise ValueError("acceptance timestamp and outcome disagree")
    agents = receipt["agents"]
    if not isinstance(agents, dict) or not agents or not all(isinstance(k, str) and k and isinstance(v, str) and v for k, v in agents.items()):
        raise ValueError("agent IDs and roles required")
    calls = receipt["calls"]
    if not isinstance(calls, list):
        raise ValueError("per-call records required")
    seen = set()
    for call in calls:
        ident = call.get("call_id")
        if not isinstance(ident, str) or not ident or ident in seen or call.get("agent_id") not in agents:
            raise ValueError("duplicate call or unknown agent")
        seen.add(ident)
        if call.get("source") != receipt["mode"] or call.get("accounting") != "per-call":
            raise ValueError("mode-bound per-call usage required; do not sum cumulative snapshots")
        for key in COUNTS:
            value = call[key]
            if value is not None and (type(value) is not int or value < 0):
                raise ValueError("token counts must be nonnegative integers or null")
        if call["input_tokens"] is not None and call["cached_input_tokens"] is not None and call["cached_input_tokens"] > call["input_tokens"]:
            raise ValueError("cached input is a subset of input")
    per_agent = {agent: {"role": role, **aggregate([call for call in calls if call["agent_id"] == agent])} for agent, role in agents.items()}
    totals = aggregate(calls)
    # Listed calls cannot prove that all runtime children were actually captured.
    complete = receipt.get("capture_complete") is True
    return {"schema_version": 1, "arm": receipt["arm"], "mode": receipt["mode"], "outcome": receipt["outcome"], "capture_complete_asserted_by_host": complete, "durations_ms": {"time_to_start": events.get("executor_ready"), "time_to_useful_action": events.get("first_useful_action"), "time_to_first_artifact": events.get("first_artifact"), "time_to_accepted_delivery": events.get("accepted"), "terminal": terminal}, "per_agent": per_agent, "all_listed_calls": totals, "run_total_tokens": totals["total_tokens"] if complete else None, "cost": None, "limitations": "Normalized caller-supplied telemetry; not proof of exhaustive runtime capture or real execution. No efficiency verdict or percentile inferred."}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("receipt")
    args = parser.parse_args()
    with Path(args.receipt).open("rb") as stream:
        data = stream.read(2_097_153)
    if len(data) > 2_097_152:
        raise ValueError("receipt exceeds 2 MiB")
    print(json.dumps(summarize(strict_json(data)), indent=2, allow_nan=False))


if __name__ == "__main__":
    main()
