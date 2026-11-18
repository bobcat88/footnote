"""Project-level coordination. Runtime launch, money and OS leases stay external."""

import hashlib
import json
import uuid
from pathlib import Path

from .contracts import STATES, encoded, file_bytes, owned, strict_json, text, validate_plan, validate_report, within
from .store import ProjectStore


def digest(data):
    return hashlib.sha256(data).hexdigest()


def usage_summary(report, activation):
    usage = report["usage"]
    actual_input, cached, output = (usage[key] for key in ("input_tokens", "cached_input_tokens", "output_tokens"))
    predicted_input = activation.get("predicted_input_tokens")
    predicted_output = activation.get("predicted_output_tokens")
    return {
        "source": usage["source"],
        "input_tokens": actual_input,
        "cached_input_tokens": cached,
        "output_tokens": output,
        "total_tokens": actual_input + output if actual_input is not None and output is not None else None,
        "uncached_input_tokens": actual_input - cached if actual_input is not None and cached is not None else None,
        "input_prediction_error": actual_input - predicted_input if actual_input is not None and predicted_input is not None else None,
        "output_prediction_error": output - predicted_output if output is not None and predicted_output is not None else None,
        "cost": None,
    }


class Governor:
    def __init__(self, database, workspace, skills_root, *, verifiers=None):
        self.verifiers = dict(verifiers or {})
        self.store = ProjectStore(database, workspace, skills_root)
        self.db = self.store.connection
        self.workspace = self.store.workspace
        self.skills_root = self.store.skills_root
        try:
            with self.store.read():
                self._validate_persisted()
        except BaseException:
            self.close()
            raise

    def close(self):
        self.store.close()

    def _validate_persisted(self):
        if self.db.execute("PRAGMA foreign_key_check").fetchone() is not None:
            raise ValueError("persisted references are corrupt")
        plan = self.store.meta("plan")
        rows = self.db.execute("SELECT id FROM tasks").fetchall()
        if plan is None:
            if rows:
                raise ValueError("task state exists without a frozen plan")
            return
        validate_plan(plan)
        if {row["id"] for row in rows} != {task["id"] for task in plan["tasks"]}:
            raise ValueError("persisted task inventory differs from frozen plan")
        for row in rows:
            current, task = self._task(row["id"])
            attempts = self.db.execute("SELECT * FROM attempts WHERE task_id=? ORDER BY rowid", (task["id"],)).fetchall()
            if current["attempts"] != len(attempts) or current["attempts"] > task["max_attempts"]:
                raise ValueError("persisted attempt count is invalid")
            if not attempts and current["state"] != "pending":
                raise ValueError("task state requires a recorded attempt")
            for attempt in attempts:
                if attempt["state"] not in {"running", "uncertain", "refused", "reported", "verifying", "verification_uncertain", "verified", "accepted", "rejected"}:
                    raise ValueError("unknown persisted attempt state")
                brief = strict_json(attempt["brief"])
                if brief.get("task_id") != task["id"] or brief.get("attempt_id") != attempt["id"] or encoded(brief.get("contract")) != encoded(task) or attempt["actor"] != task["owner"] or attempt["mode"] not in ("fixture", "real"):
                    raise ValueError("persisted attempt binding is invalid")
                if attempt["activation"]:
                    self._validate_activation(strict_json(attempt["activation"]), brief, attempt["mode"])
                if attempt["report"]:
                    validate_report(strict_json(attempt["report"]), task, attempt["id"], attempt["mode"])
                    if not attempt["activation"] or not attempt["evidence"]:
                        raise ValueError("persisted report lacks activation/evidence")
                if attempt["state"] in {"reported", "verifying", "verification_uncertain", "verified", "accepted", "rejected"} and not attempt["report"]:
                    raise ValueError("persisted attempt state requires a report")
                verification = self.db.execute("SELECT * FROM verifications WHERE attempt_id=?", (attempt["id"],)).fetchone()
                if (attempt["state"] in {"verifying", "verification_uncertain"} or "verifier" in task and attempt["state"] in {"verified", "accepted"}) and verification is None:
                    raise ValueError("verification phase lacks a recorded intent")
                if verification is not None:
                    request = strict_json(verification["request"])
                    evidence = strict_json(attempt["evidence"])
                    bound = request.get("evidence")
                    # A review adds its own evidence after verification.
                    evidence_matches = isinstance(bound, dict) and bool(bound) and all(evidence.get(k) == v for k, v in bound.items()) and (evidence == bound or attempt["state"] == "accepted" and task["review_required"])
                    if request.get("task_id") != task["id"] or request.get("workspace") != str(self.workspace) or request.get("attempt_id") != attempt["id"] or request.get("verification_id") != verification["id"] or request.get("verifier") != task.get("verifier") or not evidence_matches:
                        raise ValueError("persisted verifier binding is invalid")
                    expected_phase = {"verifying": "running", "verification_uncertain": "uncertain"}.get(attempt["state"], "finished")
                    if verification["state"] != expected_phase:
                        raise ValueError("persisted verifier phases disagree")
                    if verification["state"] == "finished":
                        result = strict_json(verification["result"])
                        self._validate_verifier_result(result, request)
                        if attempt["state"] in {"verified", "accepted"} and not result["passed"]:
                            raise ValueError("accepted state has a failed named verifier")
            if attempts:
                expected = {"pending": set(), "running": {"running"}, "uncertain": {"uncertain"}, "reported": {"reported"}, "verifying": {"verifying"}, "verification_uncertain": {"verification_uncertain"}, "verified": {"verified"}, "accepted": {"accepted"}, "revision": {"rejected", "refused"}, "escalated": {"rejected", "refused"}}
                if attempts[-1]["state"] not in expected[current["state"]]:
                    raise ValueError("persisted task/attempt states disagree")

    def _task(self, task_id):
        row = self.db.execute("SELECT * FROM tasks WHERE id=?", (task_id,)).fetchone()
        if row is None or row["state"] not in STATES or type(row["attempts"]) is not int or row["attempts"] < 0:
            raise ValueError("unknown task or invalid persisted state")
        task = json.loads(row["body"])
        plan = self.store.meta("plan")
        expected = next((item for item in plan["tasks"] if item["id"] == task_id), None)
        if encoded(task) != encoded(expected):
            raise ValueError("persisted task differs from frozen project contract")
        return row, task

    def _attempt(self, attempt_id):
        row = self.db.execute("SELECT * FROM attempts WHERE id=?", (attempt_id,)).fetchone()
        if row is None:
            raise ValueError("unknown attempt")
        return row

    def _pack(self):
        root = self.skills_root.parent
        raw = file_bytes(root / "pack-manifest.json")
        manifest = strict_json(raw)
        if not isinstance(manifest, dict) or type(manifest.get("schema_version")) is not int or manifest["schema_version"] != 1 or not isinstance(manifest.get("files"), dict) or not isinstance(manifest.get("skills"), list):
            raise ValueError("invalid capability pack manifest")
        for name, expected in manifest["files"].items():
            path = within(root, name)
            if digest(file_bytes(path)) != expected:
                raise ValueError(f"capability artifact changed: {name}")
        return digest(raw), manifest

    def plan(self, plan):
        plan = validate_plan(plan)
        pack_hash, manifest = self._pack()
        for task in plan["tasks"]:
            if "verifier" in task:
                self._verifier(task)
            for scope in task["owns"]:
                within(self.workspace, scope)
            for criterion in task["acceptance"]:
                owned(self.workspace, criterion["path"], task["owns"])
            for context in task.get("context_files", []):
                source = within(self.workspace, context)
                if any(source == within(self.workspace, scope) or source.is_relative_to(within(self.workspace, scope)) for scope in task["owns"]):
                    raise ValueError("context files are immutable; stage a separate input snapshot before transforming an owned file")
            for skill in task.get("skills", []):
                if skill not in manifest.get("skills", []) or f"skills/{skill}/SKILL.md" not in manifest["files"]:
                    raise ValueError("selected skill is absent from verified pack")
        with self.store.write():
            if self.store.meta("plan") is not None:
                raise ValueError("project contract is frozen; use a new explicit project revision")
            self.db.execute("INSERT INTO meta VALUES ('plan',?)", (encoded(plan),))
            self.db.execute("INSERT INTO meta VALUES ('pack_hash',?)", (encoded(pack_hash),))
            for task in plan["tasks"]:
                self.db.execute("INSERT INTO tasks(id,body,state) VALUES (?,?,'pending')", (task["id"], encoded(task)))
            self.store.event(None, "project_planned", {"plan_sha256": digest(encoded(plan).encode()), "pack_sha256": pack_hash})

    def _latest(self, task_id):
        return self.db.execute("SELECT * FROM attempts WHERE task_id=? ORDER BY rowid DESC LIMIT 1", (task_id,)).fetchone()

    def _fresh(self, attempt, checked=None):
        if attempt is None or not attempt["evidence"]:
            return False
        checked = {} if checked is None else checked
        if attempt["id"] in checked:
            return checked[attempt["id"]]
        checked[attempt["id"]] = False
        try:
            evidence = json.loads(attempt["evidence"])
            if not isinstance(evidence, dict) or not evidence or not all(digest(file_bytes(within(self.workspace, path))) == sha for path, sha in evidence.items()):
                return False
            brief = json.loads(attempt["brief"])
            if not self._context_fresh(brief):
                return False
            for dependency in brief.get("dependencies", []):
                if not self._fresh(self._attempt(dependency["attempt_id"]), checked):
                    return False
            checked[attempt["id"]] = True
            return True
        except (OSError, ValueError):
            return False

    def _context_fresh(self, brief):
        for context in brief.get("context", []):
            relative_path = Path(context["path"]).relative_to(self.workspace).as_posix()
            source = within(self.workspace, relative_path)
            if digest(file_bytes(source)) != context["sha256"]:
                return False
        return True

    def _launch_inputs_fresh(self, brief):
        pack_hash, _ = self._pack()
        if pack_hash != brief["pack_sha256"] or not self._context_fresh(brief):
            raise ValueError("launch input or capability pack changed during activation")
        checked = {}
        for dependency in brief["dependencies"]:
            row, _ = self._task(dependency["task_id"])
            attempt = self._latest(dependency["task_id"])
            if row["state"] != "accepted" or attempt is None or attempt["id"] != dependency["attempt_id"] or not self._fresh(attempt, checked):
                raise ValueError("dependency evidence changed during activation")

    def _brief(self, task, attempt_id, mode, runtime):
        pack_hash, manifest = self._pack()
        if pack_hash != self.store.meta("pack_hash"):
            raise ValueError("capability pack changed since planning")
        context = []
        for path in task.get("context_files", []):
            source = within(self.workspace, path)
            context.append({"path": str(source), "sha256": digest(file_bytes(source))})
        dependencies = []
        checked = {}
        for dependency in task.get("dependencies", []):
            row, _ = self._task(dependency)
            previous = self._latest(dependency)
            if row["state"] != "accepted" or not self._fresh(previous, checked):
                raise ValueError("dependency is not accepted with fresh evidence")
            dependencies.append({"task_id": dependency, "attempt_id": previous["id"], "evidence": json.loads(previous["evidence"])})
        skills = [{"name": name, "path": str(self.skills_root / name / "SKILL.md"), "sha256": manifest["files"][f"skills/{name}/SKILL.md"]} for name in task.get("skills", [])]
        brief = {"schema_version": 1, "task_id": task["id"], "attempt_id": attempt_id, "mode": mode, "runtime": runtime, "workspace": str(self.workspace), "contract": task, "project_decisions": self.store.meta("plan").get("decisions", []), "skills": skills, "pack_sha256": pack_hash, "context": context, "dependencies": dependencies}
        if len(encoded(brief).encode()) > 65_536:
            raise ValueError("handoff exceeds 64 KiB bound; reduce relevant context references")
        return brief

    def dispatch(self, task_id, executor, predictor=None):
        mode = getattr(executor, "mode", None)
        actor = getattr(executor, "actor", None)
        if mode not in ("fixture", "real"):
            raise ValueError("executor must declare fixture or real mode")
        if predictor is None:
            with self.store.write():
                self.store.event(task_id, "dispatch_refused", {"reason": "token layer unavailable", "mode": mode})
            raise ValueError("token layer is required for every specialist dispatch")
        if getattr(predictor, "mode", None) != mode:
            raise ValueError("predictor/executor mode mismatch")
        runtime = {"adapter": getattr(executor, "name", type(executor).__name__), "version": getattr(executor, "version", "fixture" if mode == "fixture" else None), "engine": getattr(executor, "engine", "fixture:no-model" if mode == "fixture" else None)}
        for key, value in runtime.items():
            text(value, f"runtime {key}")
        with self.store.write():
            row, task = self._task(task_id)
            if row["state"] not in ("pending", "revision") or row["attempts"] >= task["max_attempts"]:
                raise ValueError("task is not dispatchable; uncertain work cannot be retried")
            if actor != task["owner"]:
                raise ValueError("executor does not match task owner")
            for active in self.db.execute("SELECT id FROM tasks WHERE state IN ('running','uncertain','reported','verifying','verification_uncertain','verified')").fetchall():
                _, other = self._task(active["id"])
                for scope in task["owns"]:
                    left = within(self.workspace, scope)
                    for other_scope in other["owns"]:
                        right = within(self.workspace, other_scope)
                        if left == right or left.is_relative_to(right) or right.is_relative_to(left):
                            raise ValueError("task ownership overlaps active or uncertain work")
            attempt_id = str(uuid.uuid4())
            brief = self._brief(task, attempt_id, mode, runtime)
            self.db.execute("UPDATE tasks SET state='running', attempts=attempts+1 WHERE id=?", (task_id,))
            self.db.execute("INSERT INTO attempts(id,task_id,actor,mode,brief,state) VALUES (?,?,?,?,?,'running')", (attempt_id, task_id, actor, mode, encoded(brief)))
            self.store.event(task_id, "dispatch_intent", {"attempt_id": attempt_id, "mode": mode, "brief_sha256": digest(encoded(brief).encode())})
        # The callable boundaries are trusted host integrations, not model-controlled reports.
        # No transaction spans a runtime or token-layer call.
        try:
            activation = predictor.activate(json.loads(encoded(brief)))
            self._validate_activation(activation, brief, mode)
            with self.store.write():
                if self._attempt(attempt_id)["state"] != "running" or self._task(task_id)[0]["state"] != "running":
                    raise ValueError("dispatch state changed during activation; executor not launched")
                self._launch_inputs_fresh(brief)
                self.db.execute("UPDATE attempts SET activation=? WHERE id=?", (encoded(activation), attempt_id))
                self.store.event(task_id, "token_activated", {"attempt_id": attempt_id, "receipt": activation})
        except Exception as error:
            with self.store.write():
                if self._attempt(attempt_id)["state"] == "running" and self._task(task_id)[0]["state"] == "running":
                    self.db.execute("UPDATE attempts SET state='refused' WHERE id=?", (attempt_id,))
                    self._revision(task_id, task, "token_activation_refused", {"attempt_id": attempt_id, "error_type": type(error).__name__})
            raise
        try:
            report = executor.execute(json.loads(encoded(brief)), json.loads(encoded(activation)))
            self.submit_report(attempt_id, report, actor=actor)
        except Exception as error:
            with self.store.write():
                current = self._attempt(attempt_id)
                if current["state"] == "running":
                    self.db.execute("UPDATE tasks SET state='uncertain' WHERE id=?", (task_id,))
                    self.db.execute("UPDATE attempts SET state='uncertain' WHERE id=?", (attempt_id,))
                    self.store.event(task_id, "execution_uncertain", {"attempt_id": attempt_id, "error_type": type(error).__name__})
            raise
        return attempt_id

    @staticmethod
    def _validate_activation(receipt, brief, mode):
        if not isinstance(receipt, dict) or receipt.get("active") is not True or receipt.get("mode") != mode:
            raise ValueError("inactive or wrong-mode token activation")
        if receipt.get("task_id") != brief["task_id"] or receipt.get("attempt_id") != brief["attempt_id"] or receipt.get("brief_sha256") != digest(encoded(brief).encode()):
            raise ValueError("token activation is not bound to this handoff")
        for key in ("adapter", "version", "activation_id"):
            text(receipt.get(key), key)
        for key in ("predicted_input_tokens", "predicted_output_tokens"):
            value = receipt.get(key)
            if value is not None and (type(value) is not int or value < 0):
                raise ValueError("predicted usage must be nonnegative integers or null")
        encoded(receipt)

    def submit_report(self, attempt_id, report, *, actor):
        with self.store.write():
            attempt = self._attempt(attempt_id)
            row, task = self._task(attempt["task_id"])
            if attempt["state"] not in ("running", "uncertain") or row["state"] not in ("running", "uncertain") or actor != attempt["actor"] or not attempt["activation"]:
                raise ValueError("report is duplicate, unauthorized or not dispatch-bound")
            report = validate_report(report, task, attempt_id, attempt["mode"])
            evidence = {}
            for path in report["artifacts"]:
                source = owned(self.workspace, path, task["owns"])
                evidence[source.relative_to(self.workspace).as_posix()] = digest(file_bytes(source))
            self.db.execute("UPDATE attempts SET state='reported', report=?, evidence=? WHERE id=?", (encoded(report), encoded(evidence), attempt_id))
            self.db.execute("UPDATE tasks SET state='reported' WHERE id=?", (task["id"],))
            self.store.event(task["id"], "report_received", {"attempt_id": attempt_id, "evidence": evidence, "usage": usage_summary(report, json.loads(attempt["activation"]))})

    def _revision(self, task_id, task, event, payload):
        attempts = self.db.execute("SELECT attempts FROM tasks WHERE id=?", (task_id,)).fetchone()[0]
        state = "revision" if attempts < task["max_attempts"] else "escalated"
        self.db.execute("UPDATE tasks SET state=? WHERE id=?", (state, task_id))
        self.store.event(task_id, event, {**payload, "next_state": state})
        return state

    def _verifier(self, task):
        descriptor = task["verifier"]
        verifier = self.verifiers.get(descriptor["id"])
        if verifier is None or getattr(verifier, "descriptor", None) != descriptor or not callable(getattr(verifier, "run", None)):
            raise ValueError("named verifier is unavailable or differs from the frozen host registration")
        return verifier

    @staticmethod
    def _validate_verifier_result(result, request):
        if not isinstance(result, dict) or set(result) != {"verification_id", "request_sha256", "passed", "details"}:
            raise ValueError("invalid trusted verifier receipt")
        if result["verification_id"] != request["verification_id"] or result["request_sha256"] != digest(encoded(request).encode()) or type(result["passed"]) is not bool or not isinstance(result["details"], dict):
            raise ValueError("verifier receipt is not bound to this run")
        encoded(result)

    def verify(self, task_id):
        verifier = request = None
        with self.store.write():
            row, task = self._task(task_id)
            attempt = self._latest(task_id)
            if row["state"] != "reported" or attempt is None or attempt["state"] != "reported":
                raise ValueError("task is not ready for verification")
            report = json.loads(attempt["report"])
            failures = []
            if not self._fresh(attempt):
                failures.append("artifact evidence changed or disappeared")
            if report.get("unresolved"):
                failures.append("reported unresolved issues require correction")
            evidence = json.loads(attempt["evidence"])
            for criterion in task["acceptance"]:
                try:
                    path = owned(self.workspace, criterion["path"], task["owns"])
                    data = file_bytes(path)
                    if evidence.get(path.relative_to(self.workspace).as_posix()) != digest(data):
                        raise ValueError("required artifact is not bound to the report")
                    if criterion["kind"] == "contains":
                        passed = criterion["value"] in data.decode("utf-8")
                    else:
                        passed = encoded(strict_json(data)) == encoded(criterion["value"])
                    if not passed:
                        failures.append(f"criterion failed: {criterion['path']}")
                except (OSError, ValueError, RecursionError):
                    failures.append(f"unreadable or unbound criterion: {criterion['path']}")
            if failures:
                self.db.execute("UPDATE attempts SET state='rejected' WHERE id=?", (attempt["id"],))
                return self._revision(task_id, task, "verification_failed", {"attempt_id": attempt["id"], "failures": failures})
            if "verifier" in task:
                verifier = self._verifier(task)
                request = {"verification_id": str(uuid.uuid4()), "task_id": task_id, "attempt_id": attempt["id"], "workspace": str(self.workspace), "verifier": task["verifier"], "evidence": evidence}
                self.db.execute("INSERT INTO verifications VALUES (?,?,'running',?,NULL)", (request["verification_id"], attempt["id"], encoded(request)))
                self.db.execute("UPDATE tasks SET state='verifying' WHERE id=?", (task_id,))
                self.db.execute("UPDATE attempts SET state='verifying' WHERE id=?", (attempt["id"],))
                self.store.event(task_id, "verification_intent", request)
            else:
                return self._accept_verified(task_id, task, attempt, evidence)
        # Trusted host code may execute tests. No write transaction spans this call.
        try:
            result = verifier.run(json.loads(encoded(request)))
            return self._finish_verification(request["verification_id"], result)
        except BaseException:
            with self.store.write():
                run = self.db.execute("SELECT state FROM verifications WHERE id=?", (request["verification_id"],)).fetchone()
                if run["state"] == "running":
                    self._verification_uncertain(request, "verifier interrupted or returned an invalid receipt")
            raise

    def _accept_verified(self, task_id, task, attempt, evidence):
        state = "verified" if task["review_required"] else "accepted"
        self.db.execute("UPDATE tasks SET state=? WHERE id=?", (state, task_id))
        self.db.execute("UPDATE attempts SET state=? WHERE id=?", (state, attempt["id"]))
        self.store.event(task_id, "verification_passed", {"attempt_id": attempt["id"], "state": state, "criteria": task["acceptance"], "evidence": evidence})
        return state

    def _verification_uncertain(self, request, reason):
        self.db.execute("UPDATE verifications SET state='uncertain' WHERE id=?", (request["verification_id"],))
        self.db.execute("UPDATE attempts SET state='verification_uncertain' WHERE id=?", (request["attempt_id"],))
        self.db.execute("UPDATE tasks SET state='verification_uncertain' WHERE id=?", (request["task_id"],))
        self.store.event(request["task_id"], "verification_uncertain", {"verification_id": request["verification_id"], "reason": reason, "automatic_retry": False})

    def mark_verification_uncertain(self, verification_id, *, actor, reason):
        text(actor, "reconciliation actor")
        text(reason, "uncertainty reason")
        with self.store.write():
            run = self.db.execute("SELECT * FROM verifications WHERE id=?", (verification_id,)).fetchone()
            if run is None or run["state"] != "running":
                raise ValueError("only an active verifier can be marked uncertain")
            self._verification_uncertain(strict_json(run["request"]), reason)
            self.store.event(None, "verification_observed", {"verification_id": verification_id, "actor": actor})

    def reconcile_verification(self, verification_id, result, *, actor, rationale):
        text(actor, "reconciliation actor")
        text(rationale, "reconciliation rationale")
        return self._finish_verification(verification_id, result, actor=actor, rationale=rationale)

    def _finish_verification(self, verification_id, result, *, actor=None, rationale=None):
        with self.store.write():
            run = self.db.execute("SELECT * FROM verifications WHERE id=?", (verification_id,)).fetchone()
            required = "uncertain" if actor is not None else "running"
            if run is None or run["state"] != required:
                raise ValueError("verifier result is duplicate or not in the required phase")
            request = strict_json(run["request"])
            self._validate_verifier_result(result, request)
            row, task = self._task(request["task_id"])
            attempt = self._latest(task["id"])
            expected = "verification_uncertain" if actor is not None else "verifying"
            if row["state"] != expected or attempt["id"] != request["attempt_id"] or attempt["state"] != expected:
                raise ValueError("verifier attempt binding changed")
            failures = []
            if not result["passed"]:
                failures.append("trusted named test suite failed")
            try:
                self._launch_inputs_fresh(strict_json(attempt["brief"]))
                if not self._fresh(attempt):
                    failures.append("artifact or context changed during verification")
            except (OSError, ValueError):
                failures.append("launch inputs changed during verification")
            self.db.execute("UPDATE verifications SET state='finished', result=? WHERE id=?", (encoded(result), verification_id))
            self.store.event(task["id"], "named_verifier_result", {"verification_id": verification_id, "result": result, "reconciled_by": actor, "rationale": rationale})
            if failures:
                self.db.execute("UPDATE attempts SET state='rejected' WHERE id=?", (attempt["id"],))
                return self._revision(task["id"], task, "verification_failed", {"attempt_id": attempt["id"], "failures": failures})
            return self._accept_verified(task["id"], task, attempt, strict_json(attempt["evidence"]))

    def review(self, task_id, *, reviewer, approve, evidence_path, rationale):
        text(reviewer, "reviewer")
        text(rationale, "review rationale")
        if type(approve) is not bool:
            raise ValueError("review decision must be explicit")
        path = within(self.workspace, evidence_path)
        review_hash = digest(file_bytes(path))
        with self.store.write():
            row, task = self._task(task_id)
            attempt = self._latest(task_id)
            if row["state"] != "verified" or attempt is None or reviewer == attempt["actor"]:
                raise ValueError("independent review required; executor cannot approve itself")
            if not self._fresh(attempt):
                raise ValueError("evidence changed after verification")
            evidence = json.loads(attempt["evidence"])
            evidence[path.relative_to(self.workspace).as_posix()] = review_hash
            if approve:
                self.db.execute("UPDATE tasks SET state='accepted' WHERE id=?", (task_id,))
                self.db.execute("UPDATE attempts SET state='accepted', evidence=? WHERE id=?", (encoded(evidence), attempt["id"]))
            else:
                self.db.execute("UPDATE attempts SET state='rejected' WHERE id=?", (attempt["id"],))
                self._revision(task_id, task, "review_rejected", {"attempt_id": attempt["id"]})
            self.store.event(task_id, "review_decided", {"attempt_id": attempt["id"], "reviewer": reviewer, "approve": approve, "rationale": rationale, "evidence_path": evidence_path, "sha256": review_hash})

    def mark_uncertain(self, attempt_id, *, actor, reason):
        text(actor, "reconciliation actor")
        text(reason, "uncertainty reason")
        with self.store.write():
            attempt = self._attempt(attempt_id)
            row, _ = self._task(attempt["task_id"])
            if row["state"] != "running" or attempt["state"] != "running":
                raise ValueError("only running attempts can be marked uncertain")
            self.db.execute("UPDATE attempts SET state='uncertain' WHERE id=?", (attempt_id,))
            self.db.execute("UPDATE tasks SET state='uncertain' WHERE id=?", (attempt["task_id"],))
            self.store.event(attempt["task_id"], "outcome_uncertain", {"attempt_id": attempt_id, "actor": actor, "reason": reason, "automatic_retry": False})

    def admit_memory(self, task_id, candidate_index, *, actor, kind, rationale, supersedes=None):
        text(actor, "memory admission actor")
        text(rationale, "memory admission rationale")
        if kind not in ("observation", "fact", "decision") or type(candidate_index) is not int or candidate_index < 0:
            raise ValueError("invalid memory classification or candidate index")
        with self.store.write():
            row, _ = self._task(task_id)
            attempt = self._latest(task_id)
            if row["state"] != "accepted" or attempt is None or actor == attempt["actor"] or not self._fresh(attempt):
                raise ValueError("memory admission needs accepted fresh evidence and a governor decision")
            report = json.loads(attempt["report"])
            candidates = report.get("candidate_memories", [])
            if candidate_index >= len(candidates):
                raise ValueError("unknown memory candidate")
            if self.db.execute("SELECT 1 FROM memories WHERE attempt_id=? AND candidate_index=?", (attempt["id"], candidate_index)).fetchone():
                raise ValueError("candidate already admitted")
            if supersedes is not None:
                old = self.db.execute("SELECT active FROM memories WHERE id=?", (supersedes,)).fetchone()
                if old is None or not old[0]:
                    raise ValueError("superseded memory must be current")
            candidate = candidates[candidate_index]
            ident = str(uuid.uuid4())
            evidence = json.loads(attempt["evidence"])
            self.db.execute("INSERT INTO memories VALUES (?,?,?,?,?,?,?,?,?,1)", (ident, attempt["id"], candidate_index, kind, candidate["text"], encoded(evidence), actor, rationale, supersedes))
            if supersedes is not None:
                self.db.execute("UPDATE memories SET active=0 WHERE id=?", (supersedes,))
            self.store.event(task_id, "memory_admitted", {"id": ident, "attempt_id": attempt["id"], "candidate_index": candidate_index, "kind": kind, "actor": actor, "rationale": rationale, "supersedes": supersedes})
            return ident

    def snapshot(self):
        with self.store.read():
            return self._snapshot()

    def _snapshot(self):
        tasks = []
        checked = {}
        for row in self.db.execute("SELECT id FROM tasks ORDER BY rowid").fetchall():
            current, task = self._task(row["id"])
            attempt = self._latest(task["id"])
            usage = usage_summary(json.loads(attempt["report"]), json.loads(attempt["activation"])) if attempt and attempt["report"] and attempt["activation"] else None
            tasks.append({"id": task["id"], "state": current["state"], "attempts": current["attempts"], "latest_attempt": attempt["id"] if attempt else None, "evidence_fresh": self._fresh(attempt, checked) if current["state"] in ("reported", "verified", "accepted") else None, "usage": usage})
        return {"schema_version": 1, "goal": (self.store.meta("plan") or {}).get("goal"), "tasks": tasks, "complete": bool(tasks) and all(task["state"] == "accepted" and task["evidence_fresh"] for task in tasks), "memories": [dict(row) for row in self.db.execute("SELECT * FROM memories ORDER BY rowid")], "events": [dict(row) for row in self.db.execute("SELECT * FROM events ORDER BY id")]}
