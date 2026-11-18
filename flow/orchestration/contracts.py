"""Validation at project, executor and token-layer boundaries."""

import json
import re
from pathlib import Path


STATES = {"pending", "running", "reported", "verifying", "verification_uncertain", "verified", "accepted", "revision", "uncertain", "escalated"}
SLUG = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")
MAX_FILE_BYTES = 1_048_576
MAX_BRIEF_BYTES = 65_536


def encoded(value):
    try:
        result = json.dumps(value, sort_keys=True, separators=(",", ":"), allow_nan=False)
    except (TypeError, ValueError, RecursionError) as error:
        raise ValueError("value is not finite, serializable JSON") from error
    if len(result.encode("utf-8")) > 2_097_152:
        raise ValueError("JSON contract exceeds 2 MiB")
    return result


def text(value, label):
    if not isinstance(value, str) or not value.strip() or len(value) > 16_384:
        raise ValueError(f"invalid {label}")


def strings(value, label, *, nonempty=False):
    if not isinstance(value, list) or len(value) > 256 or (nonempty and not value):
        raise ValueError(f"invalid {label}")
    for item in value:
        text(item, label)


def relative(value):
    text(value, "relative path")
    path = Path(value)
    if path.is_absolute() or path.drive or ".." in path.parts or path == Path("."):
        raise ValueError("path must be a nonempty relative path without traversal")
    return path


def within(root, value):
    path = (root / relative(value)).resolve()
    if not path.is_relative_to(root.resolve()):
        raise ValueError("path escapes workspace")
    return path


def owned(root, value, scopes):
    path = within(root, value)
    if not any(path == within(root, scope) or path.is_relative_to(within(root, scope)) for scope in scopes):
        raise ValueError("artifact is outside task ownership")
    return path


def file_bytes(path):
    with path.open("rb") as stream:
        data = stream.read(MAX_FILE_BYTES + 1)
    if len(data) > MAX_FILE_BYTES:
        raise ValueError("evidence file exceeds 1 MiB; use a bounded receipt")
    return data


def strict_json(data):
    def unique(pairs):
        result = {}
        for key, value in pairs:
            if key in result:
                raise ValueError("duplicate JSON keys are ambiguous")
            result[key] = value
        return result
    value = json.loads(data, object_pairs_hook=unique)
    encoded(value)
    return value


def validate_plan(plan):
    if not isinstance(plan, dict) or set(plan) - {"schema_version", "goal", "decisions", "tasks"}:
        raise ValueError("invalid project contract fields")
    if type(plan.get("schema_version")) is not int or plan["schema_version"] != 1:
        raise ValueError("unsupported project contract schema")
    text(plan.get("goal"), "project goal")
    strings(plan.get("decisions", []), "project decisions")
    tasks = plan.get("tasks")
    if not isinstance(tasks, list) or not 1 <= len(tasks) <= 256:
        raise ValueError("plan needs 1 to 256 bounded tasks")
    ids = set()
    for task in tasks:
        if not isinstance(task, dict):
            raise ValueError("task must be an object")
        fields = {"id", "objective", "owner", "owns", "acceptance", "skills", "context_files", "constraints", "permitted_actions", "stop_condition", "dependencies", "max_attempts", "review_required", "verifier"}
        if set(task) - fields:
            raise ValueError("unknown task fields")
        ident = task.get("id")
        if not isinstance(ident, str) or len(ident) > 64 or not SLUG.fullmatch(ident) or ident in ids:
            raise ValueError("task IDs must be unique slugs")
        ids.add(ident)
        for field in ("objective", "owner", "stop_condition"):
            text(task.get(field), field)
        for field in ("owns", "permitted_actions"):
            strings(task.get(field), field, nonempty=True)
        for field in ("skills", "context_files", "constraints", "dependencies"):
            strings(task.get(field, []), field)
        for path in task["owns"] + task.get("context_files", []):
            relative(path)
        for skill in task.get("skills", []):
            if not SLUG.fullmatch(skill) or len(skill) > 64:
                raise ValueError("invalid skill name")
        if type(task.get("max_attempts")) is not int or not 1 <= task["max_attempts"] <= 20:
            raise ValueError("max_attempts must be between 1 and 20")
        if type(task.get("review_required")) is not bool:
            raise ValueError("review_required must be explicit")
        if "verifier" in task:
            descriptor = task["verifier"]
            if not isinstance(descriptor, dict) or set(descriptor) != {"id", "version", "sha256"}:
                raise ValueError("verifier needs a host-registered identity, version and fingerprint")
            for key in ("id", "version"):
                text(descriptor[key], "verifier " + key)
            if not isinstance(descriptor["sha256"], str) or not re.fullmatch("[a-f0-9]{64}", descriptor["sha256"]):
                raise ValueError("invalid verifier fingerprint")
        criteria = task.get("acceptance")
        if not isinstance(criteria, list) or not 1 <= len(criteria) <= 64:
            raise ValueError("task needs explicit acceptance criteria")
        for criterion in criteria:
            if not isinstance(criterion, dict) or set(criterion) != {"kind", "path", "value"}:
                raise ValueError("invalid acceptance criterion")
            relative(criterion["path"])
            if criterion["kind"] == "contains":
                text(criterion["value"], "required artifact text")
            elif criterion["kind"] == "json_equals":
                encoded(criterion["value"])
            else:
                raise ValueError("unsupported verifier; arbitrary commands are not accepted")
    pending = {task["id"]: set(task.get("dependencies", [])) for task in tasks}
    if any(not dependencies <= ids for dependencies in pending.values()):
        raise ValueError("unknown dependency")
    done = set()
    while pending:
        ready = {ident for ident, dependencies in pending.items() if dependencies <= done}
        if not ready:
            raise ValueError("cyclic task dependencies")
        done.update(ready)
        pending = {ident: dependencies for ident, dependencies in pending.items() if ident not in ready}
    return json.loads(encoded(plan))


def validate_report(report, task, attempt_id, mode):
    fields = {"task_id", "attempt_id", "summary", "artifacts", "unresolved", "usage", "candidate_memories", "runtime_turn_id"}
    if not isinstance(report, dict) or set(report) - fields:
        raise ValueError("unknown report fields; executor reports cannot approve themselves")
    if report.get("task_id") != task["id"] or report.get("attempt_id") != attempt_id:
        raise ValueError("report identity mismatch")
    text(report.get("summary"), "report summary")
    strings(report.get("artifacts"), "reported artifacts", nonempty=True)
    strings(report.get("unresolved", []), "unresolved issues")
    usage = report.get("usage")
    if not isinstance(usage, dict) or set(usage) != {"source", "input_tokens", "cached_input_tokens", "output_tokens"}:
        raise ValueError("usage must declare source and nullable token counts")
    if usage["source"] not in ("runtime", "fixture", "unknown") or (mode == "real" and usage["source"] == "fixture") or (mode == "fixture" and usage["source"] == "runtime"):
        raise ValueError("invalid usage provenance")
    for key in ("input_tokens", "cached_input_tokens", "output_tokens"):
        if usage[key] is not None and (type(usage[key]) is not int or usage[key] < 0):
            raise ValueError("usage must be nonnegative integers or null")
        if usage["source"] == "unknown" and usage[key] is not None:
            raise ValueError("unknown usage cannot invent counts")
    if usage["input_tokens"] is not None and usage["cached_input_tokens"] is not None and usage["cached_input_tokens"] > usage["input_tokens"]:
        raise ValueError("cached input is a subset of input tokens")
    candidates = report.get("candidate_memories", [])
    if not isinstance(candidates, list) or len(candidates) > 64:
        raise ValueError("invalid memory proposals")
    for candidate in candidates:
        if not isinstance(candidate, dict) or set(candidate) != {"text", "evidence"}:
            raise ValueError("invalid memory proposal fields")
        text(candidate["text"], "memory proposal")
        strings(candidate["evidence"], "memory evidence", nonempty=True)
        if not set(candidate["evidence"]) <= set(report["artifacts"]):
            raise ValueError("memory evidence must reference reported artifacts")
    if report.get("runtime_turn_id") is not None:
        text(report["runtime_turn_id"], "runtime turn ID")
    return json.loads(encoded(report))
