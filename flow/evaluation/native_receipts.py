"""Read one explicitly identified native child's runtime telemetry, not its reasoning."""

import json
from datetime import datetime
from pathlib import Path


def launch_time(parent_session, task_name):
    matches = []
    for line in Path(parent_session).open(encoding="utf-8"):
        record = json.loads(line)
        payload = record.get("payload", {})
        if record["type"] == "response_item" and payload.get("type") == "function_call" and payload.get("name") == "spawn_agent":
            if json.loads(payload["arguments"]).get("task_name") == task_name:
                matches.append(record["timestamp"])
    if len(matches) != 1:
        raise ValueError("one explicit native launch event required")
    return matches[0]


def read_session(path, *, agent_path, parent_id):
    records = []
    with Path(path).open(encoding="utf-8") as stream:
        first = json.loads(next(stream))
        meta = first.get("payload", {})
        if meta.get("agent_path") != agent_path or meta.get("parent_thread_id") != parent_id:
            raise ValueError("session does not belong to the selected child and parent")
        for line in stream:
            records.append(json.loads(line))
    starts, ends, calls, tool_times, provider_calls = [], [], [], [], []
    previous = {key: 0 for key in ("input_tokens", "cached_input_tokens", "output_tokens")}
    latest = None
    model = effort = None
    for record in records:
        payload = record.get("payload", {})
        if record["type"] == "token_usage_record":
            if payload.get("thread_id") != meta["id"]:
                raise ValueError("usage belongs to another native thread")
            provider_calls.append({"call_id": payload["response_id"], "agent_id": agent_path, "source": "real", "accounting": "per-call", **{key: payload["usage"][key] for key in previous}, "completed_at": record["timestamp"]})
        if record["type"] == "turn_context":
            model = payload.get("model", model)
            effort = payload.get("effort", payload.get("reasoning_effort", effort))
        if record["type"] == "response_item" and payload.get("type") in ("function_call", "custom_tool_call"):
            tool_times.append(record["timestamp"])
        if record["type"] != "event_msg":
            continue
        kind = payload.get("type")
        if kind == "task_started": starts.append(record["timestamp"])
        if kind == "task_complete": ends.append(record["timestamp"])
        if kind != "token_count" or not payload.get("info"):
            continue
        info = payload["info"]
        total = info["total_token_usage"]
        current = {key: total.get(key) for key in previous}
        if current == previous:
            continue
        last = info["last_token_usage"]
        for key, old in previous.items():
            if type(current[key]) is not int or current[key] < old or current[key] - old != last.get(key):
                raise ValueError("usage snapshots do not reconstruct distinct per-call usage")
        calls.append({"call_id": f"{meta['id']}:{len(calls) + 1}", "agent_id": agent_path, "source": "real", "accounting": "per-call", **{key: last[key] for key in previous}, "completed_at": record["timestamp"]})
        previous, latest = current, total
    if len(starts) != 1 or len(ends) != 1 or not calls:
        raise ValueError("one complete child task with runtime usage required")
    if provider_calls:
        if len({call["call_id"] for call in provider_calls}) != len(provider_calls) or any(sum(call[key] for call in provider_calls) != previous[key] for key in previous):
            raise ValueError("provider response receipts do not reconcile to cumulative usage")
        calls = provider_calls
    if any(call["cached_input_tokens"] > call["input_tokens"] for call in calls):
        raise ValueError("invalid cache accounting")
    elapsed = (datetime.fromisoformat(ends[0].replace("Z", "+00:00")) - datetime.fromisoformat(starts[0].replace("Z", "+00:00"))).total_seconds()
    return {"agent_path": agent_path, "session_id": meta["id"], "session_file": Path(path).name, "model": model, "reasoning_effort": effort, "started_at": starts[0], "completed_at": ends[0], "elapsed_seconds": elapsed, "tool_calls": len(tool_times), "first_tool_at": tool_times[0] if tool_times else None, "calls": calls, "usage_basis": "provider-response-id" if provider_calls else "reconciled-snapshot-deltas", "cumulative_usage": latest, "time_to_start_ms": None, "ttft_ms": None, "limitations": "Elapsed covers native child task, not host submission through external acceptance. First tool is not necessarily useful work. Startup/TTFT unavailable. Parent setup/coordinator reasoning excluded; not a complete full-framework efficiency receipt."}
