//! What agents do with suggested skills (#443): whether they read the `SKILL.md`, and whether the skill's scripts pass.
//! The prompt hook watches each skill it suggests; at Stop it scans only the part of the session file written since its
//! last scan and appends events to `<state>/memory-hook/skill-outcomes.jsonl`. Rows hold the skill's name and theme, the
//! project and the event, never a prompt, path or command. Free: no tokens, no model. Claude Code session files only.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Past this size the older half of the log is dropped, as for `activity.jsonl`.
pub const MAX_LOG: u64 = 512 * 1024;

/// A skill suggested in this session, watched in the session file.
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub struct Watch {
    pub name: String,
    pub theme: String,
    /// The skill's folder (the one holding its `SKILL.md`).
    pub dir: String,
    /// Already counted as read since it was last suggested.
    #[serde(default)]
    pub read: bool,
}

/// What a watched skill went through, in session-file order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Seen {
    Read,
    Run { ok: bool },
}

pub fn log_path(state: &Path) -> PathBuf {
    state.join("memory-hook").join("skill-outcomes.jsonl")
}

fn norm(s: &str) -> String {
    s.replace('\\', "/").to_lowercase()
}

/// The skill's folder as written in paths and commands: absolute (normalized), and its last component.
fn keys(w: &Watch) -> (String, String) {
    let dir = norm(&w.dir).trim_end_matches('/').to_string();
    let folder = dir.rsplit('/').next().unwrap_or_default().to_string();
    (dir, folder)
}

fn reads_skill(path: &str, dir: &str, folder: &str) -> bool {
    path == format!("{dir}/skill.md") || path.ends_with(&format!("/{folder}/skill.md"))
}

fn runs_script(cmd: &str, dir: &str, folder: &str) -> bool {
    cmd.contains(&format!("{dir}/scripts/")) || cmd.contains(&format!("{folder}/scripts/"))
}

/// A tool result that failed: flagged as an error, or a shell result that starts with a non-zero exit code.
fn failed(result: &Value) -> bool {
    if result["is_error"].as_bool() == Some(true) {
        return true;
    }
    let text = match &result["content"] {
        Value::String(s) => s.as_str(),
        Value::Array(parts) => parts
            .iter()
            .find_map(|p| p["text"].as_str())
            .unwrap_or_default(),
        _ => "",
    };
    text.trim_start().starts_with("Exit code ") && !text.trim_start().starts_with("Exit code 0")
}

/// Reads and script runs of the watched skills in a slice of a Claude Code session file (JSONL), as `(watch index,
/// what)`. Only tool names, the read path, the command and the result's error flag are looked at; nothing is kept.
/// A run whose result is not in the slice is not counted.
pub fn scan(chunk: &[u8], watches: &[Watch]) -> Vec<(usize, Seen)> {
    let keys: Vec<(String, String)> = watches.iter().map(keys).collect();
    let mut out = vec![];
    let mut running: HashMap<String, usize> = HashMap::new();
    for line in chunk.split(|&b| b == b'\n') {
        let has = |needle: &[u8]| line.windows(needle.len()).any(|w| w == needle);
        if !has(b"\"tool_use\"") && !has(b"\"tool_result\"") {
            continue;
        }
        let Ok(v) = serde_json::from_slice::<Value>(line) else {
            continue;
        };
        let Some(parts) = v["message"]["content"].as_array() else {
            continue;
        };
        for p in parts {
            match p["type"].as_str() {
                Some("tool_use") => {
                    let input = &p["input"];
                    if p["name"] == "Read" {
                        let path = norm(input["file_path"].as_str().unwrap_or_default());
                        if let Some(i) = keys.iter().position(|(d, f)| reads_skill(&path, d, f)) {
                            out.push((i, Seen::Read));
                        }
                    } else if let Some(cmd) = input["command"].as_str() {
                        let cmd = norm(cmd);
                        for (i, (d, f)) in keys.iter().enumerate() {
                            if cmd.contains(&format!("{d}/skill.md"))
                                || cmd.contains(&format!("/{f}/skill.md"))
                            {
                                out.push((i, Seen::Read));
                            } else if runs_script(&cmd, d, f)
                                && let Some(id) = p["id"].as_str()
                            {
                                running.insert(id.to_string(), i);
                                break;
                            }
                        }
                    }
                }
                Some("tool_result") => {
                    if let Some(i) = p["tool_use_id"].as_str().and_then(|id| running.remove(id)) {
                        out.push((i, Seen::Run { ok: !failed(p) }));
                    }
                }
                _ => {}
            }
        }
    }
    out
}

/// Appends one event row, dropping the older half of the log past `MAX_LOG`. Best effort.
pub fn append(state: &Path, project: &str, w: &Watch, event: &str, ok: Option<bool>) {
    let path = log_path(state);
    if fs::metadata(&path).is_ok_and(|m| m.len() > MAX_LOG)
        && let Ok(text) = fs::read_to_string(&path)
    {
        let lines: Vec<&str> = text.lines().collect();
        let _ = fs::write(&path, lines[lines.len() / 2..].join("\n") + "\n");
    }
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64);
    let mut row =
        json!({"t": t, "project": project, "skill": w.name, "theme": w.theme, "event": event});
    if let Some(ok) = ok {
        row["ok"] = json!(ok);
    }
    let _ = footnote_core::state::append_line(&path, &row.to_string());
}

/// `footnote skills stats`: per skill, how often it was suggested, read, and how its scripts ran.
pub fn stats(state: &Path) -> Result<Value> {
    let text = fs::read_to_string(log_path(state)).unwrap_or_default();
    let mut by: Map<String, Value> = Map::new();
    for v in text
        .lines()
        .filter_map(|l| serde_json::from_str::<Value>(l).ok())
    {
        let Some(name) = v["skill"].as_str() else {
            continue;
        };
        let row = by.entry(name).or_insert_with(|| {
            json!({"skill": name, "theme": v["theme"], "suggested": 0, "read": 0, "runs": 0, "failures": 0})
        });
        let bump = |row: &mut Value, k: &str| row[k] = json!(row[k].as_u64().unwrap_or(0) + 1);
        match v["event"].as_str() {
            Some("suggested") => bump(row, "suggested"),
            Some("read") => bump(row, "read"),
            Some("run") => {
                bump(row, "runs");
                if v["ok"] == false {
                    bump(row, "failures");
                }
            }
            _ => {}
        }
    }
    let mut skills: Vec<Value> = by.into_iter().map(|(_, v)| v).collect();
    skills.sort_by(|a, b| {
        b["suggested"]
            .as_u64()
            .cmp(&a["suggested"].as_u64())
            .then_with(|| a["skill"].as_str().cmp(&b["skill"].as_str()))
    });
    Ok(json!({"skills": skills}))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn watch(dir: &str) -> Watch {
        Watch {
            name: "debug-and-test".into(),
            theme: "dev".into(),
            dir: dir.into(),
            read: false,
        }
    }

    fn line(v: Value) -> String {
        v.to_string() + "\n"
    }

    fn tool_use(id: &str, name: &str, input: Value) -> String {
        line(
            json!({"type": "assistant", "message": {"content": [{"type": "tool_use", "id": id, "name": name, "input": input}]}}),
        )
    }

    fn tool_result(id: &str, is_error: bool, text: &str) -> String {
        line(
            json!({"type": "user", "message": {"content": [{"type": "tool_result", "tool_use_id": id, "is_error": is_error, "content": text}]}}),
        )
    }

    #[test]
    fn reads_and_script_runs_are_found_with_their_result() {
        let w = [watch(r"C:\Skills\debug-and-test")];
        let chunk = [
            tool_use(
                "a",
                "Read",
                json!({"file_path": "C:/skills/debug-and-test/SKILL.md"}),
            ),
            tool_use(
                "b",
                "Bash",
                json!({"command": "node skills/debug-and-test/scripts/red-green/red-green.mjs"}),
            ),
            tool_result("b", true, "Exit code 1\nfailed"),
            tool_use(
                "c",
                "PowerShell",
                json!({"command": r"node C:\Skills\debug-and-test\scripts\leftover-check.mjs"}),
            ),
            tool_result("c", false, "ok"),
            tool_use("d", "Bash", json!({"command": "cargo test"})),
            tool_result("d", true, "Exit code 101"),
        ]
        .concat();
        assert_eq!(
            scan(chunk.as_bytes(), &w),
            vec![
                (0, Seen::Read),
                (0, Seen::Run { ok: false }),
                (0, Seen::Run { ok: true })
            ]
        );
    }

    #[test]
    fn a_shell_read_counts_and_a_run_without_its_result_does_not() {
        let w = [watch("/home/u/skills/debug-and-test")];
        let chunk = [
            tool_use(
                "a",
                "Bash",
                json!({"command": "cat /home/u/skills/debug-and-test/SKILL.md"}),
            ),
            tool_use(
                "b",
                "Bash",
                json!({"command": "python debug-and-test/scripts/x.py"}),
            ),
        ]
        .concat();
        assert_eq!(scan(chunk.as_bytes(), &w), vec![(0, Seen::Read)]);
    }

    #[test]
    fn exit_codes_in_results_decide_failure() {
        assert!(failed(&json!({"content": "Exit code 2\nboom"})));
        assert!(!failed(&json!({"content": "Exit code 0"})));
        assert!(failed(&json!({"is_error": true, "content": "x"})));
        assert!(failed(
            &json!({"content": [{"type": "text", "text": "Exit code 1"}]})
        ));
        assert!(!failed(&json!({"content": "all good"})));
    }

    #[test]
    fn stats_add_up_events() {
        let t = tempfile::tempdir().unwrap();
        fs::create_dir_all(t.path().join("memory-hook")).unwrap();
        let w = watch("/s/debug-and-test");
        append(t.path(), "p", &w, "suggested", None);
        append(t.path(), "p", &w, "suggested", None);
        append(t.path(), "p", &w, "read", None);
        append(t.path(), "p", &w, "run", Some(false));
        append(t.path(), "p", &w, "run", Some(true));
        let s = stats(t.path()).unwrap();
        assert_eq!(
            s["skills"][0],
            json!({"skill": "debug-and-test", "theme": "dev", "suggested": 2, "read": 1, "runs": 2, "failures": 1})
        );
    }
}
