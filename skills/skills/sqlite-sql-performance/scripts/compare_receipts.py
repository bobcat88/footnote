"""Compare matched benchmark receipts; no orchestration, network or subprocesses."""

import argparse
import json
import math
import statistics
import sys
from pathlib import Path


MATCH_FIELDS = ("workload_id", "environment_id", "metric", "unit", "lower_is_better")


def validate(receipt):
    if not isinstance(receipt, dict) or type(receipt.get("schema_version")) is not int:
        raise ValueError("receipt must be an object with integer schema_version")
    if receipt["schema_version"] != 1:
        raise ValueError("unsupported schema_version")
    for field in (*MATCH_FIELDS[:-1], "revision"):
        if not isinstance(receipt.get(field), str) or not receipt[field].strip():
            raise ValueError(f"missing or invalid {field}")
    if type(receipt.get("lower_is_better")) is not bool:
        raise ValueError("lower_is_better must be a boolean")
    if receipt.get("correctness_passed") is not True:
        raise ValueError("correctness must be checked and passed before comparison")
    samples = receipt.get("samples")
    if not isinstance(samples, list) or not samples or len(samples) > 100_000:
        raise ValueError("samples must contain 1 to 100000 observations")
    if any(type(x) not in (int, float) or not math.isfinite(x) or x < 0 for x in samples):
        raise ValueError("samples must be finite nonnegative numbers, not booleans")


def compare(baseline, candidate, max_regression_pct=None):
    validate(baseline)
    validate(candidate)
    for field in MATCH_FIELDS:
        if baseline[field] != candidate[field]:
            raise ValueError(f"incomparable receipts: {field} differs")
    if max_regression_pct is not None and (
        type(max_regression_pct) not in (int, float)
        or not math.isfinite(max_regression_pct)
        or max_regression_pct < 0
    ):
        raise ValueError("regression threshold must be finite and nonnegative")
    before = statistics.median(baseline["samples"])
    after = statistics.median(candidate["samples"])
    if not math.isfinite(before) or not math.isfinite(after):
        raise ValueError("median overflow; rescale measurement units")
    direction = 1 if baseline["lower_is_better"] else -1
    regression = direction * (after - before) / before * 100 if before else None
    if regression is not None and not math.isfinite(regression):
        raise ValueError("relative change overflow; rescale measurement units")
    if max_regression_pct is None:
        verdict = "measurement-only"
    elif regression is None:
        verdict = "inconclusive-zero-baseline"
    else:
        verdict = "within-budget" if regression <= max_regression_pct else "over-budget"
    return {
        "schema_version": 1,
        "verdict": verdict,
        "metric": baseline["metric"],
        "unit": baseline["unit"],
        "baseline": baseline,
        "candidate": candidate,
        "baseline_median": before,
        "candidate_median": after,
        "regression_pct": regression,
        "max_regression_pct": max_regression_pct,
        "statistical_significance": "not evaluated",
        "limitations": "Matching metadata and correctness assertions are supplied by the caller; no independent verification. Medians do not establish tail latency or statistical significance.",
    }


def unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise ValueError("duplicate JSON keys are ambiguous")
        result[key] = value
    return result


def load(path):
    # Receipts are small metadata files, not arbitrary benchmark payloads.
    with Path(path).open("rb") as stream:
        data = stream.read(1_048_577)
    if len(data) > 1_048_576:
        raise ValueError("receipt exceeds 1 MiB input limit")
    return json.loads(data.decode("utf-8-sig"), object_pairs_hook=unique_object)


def main(argv=None):
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("baseline")
    parser.add_argument("candidate")
    parser.add_argument("--max-regression-pct", type=float)
    args = parser.parse_args(argv)
    try:
        report = compare(load(args.baseline), load(args.candidate), args.max_regression_pct)
        print(json.dumps(report, indent=2, allow_nan=False))
    except (OSError, ValueError, OverflowError, RecursionError) as error:
        print(f"Invalid comparison: {error}", file=sys.stderr)
        return 2
    return 0 if report["verdict"] in ("measurement-only", "within-budget") else 1


if __name__ == "__main__":
    raise SystemExit(main())
