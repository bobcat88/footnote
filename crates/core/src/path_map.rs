//! Opt-in host-to-container path translation (`FOOTNOTE_PATH_MAP`).
//!
//! When Footnote runs inside a Linux container on a Windows (or macOS/Linux) host,
//! agent payloads (`footnote_code` `root`, hook `cwd` and `transcript_path`) may carry
//! host paths such as `C:\Users\<user>\repos\footnote` while the repository is mounted at
//! `/workspaces/footnote`.
//!
//! Operators configure `FOOTNOTE_PATH_MAP` with semicolon-separated `HOST=CONTAINER`
//! prefix pairs (semicolon avoids colliding with Windows drive colons):
//!
//! ```text
//! FOOTNOTE_PATH_MAP=C:\Users\<user>\repos=/workspaces;C:\Users\<user>\.claude=/host-claude
//! ```
//!
//! Security properties:
//! - Configured exclusively via operator environment variable, never via MCP input.
//! - Matches on full path-segment boundaries (`C:/work/app` never matches `C:/work/app2`).
//! - Longest matching host prefix wins when rules overlap.
//! - Rejects any `..` traversal component or NUL byte in the relative tail (fail-closed:
//!   returns the original input unchanged so downstream `known_project()` / `no_links()`
//!   refuse it).
//! - Container targets must be absolute POSIX paths (`/…`) containing no `..` or `.` segments.
//! - When `FOOTNOTE_PATH_MAP` is unset or empty, this is a zero-cost no-op.

/// Translate a host path into its container path using `FOOTNOTE_PATH_MAP`.
///
/// Returns `raw` unchanged when `FOOTNOTE_PATH_MAP` is unset/empty, no prefix matches,
/// or the relative tail contains traversal components (`..`) or NUL bytes.
#[must_use]
pub fn map_host_path(raw: &str) -> String {
    let Ok(spec) = std::env::var("FOOTNOTE_PATH_MAP") else {
        return raw.to_string();
    };
    map_with_spec(raw, &spec)
}

/// Pure translation helper for `FOOTNOTE_PATH_MAP` specifications (testable without env mutation).
#[must_use]
pub fn map_with_spec(raw: &str, spec: &str) -> String {
    if spec.trim().is_empty() || raw.trim().is_empty() || raw.contains('\0') {
        return raw.to_string();
    }
    let input = normalize_slashes(raw);
    let mut best: Option<(usize, String)> = None;

    for entry in spec.split(';') {
        let Some((host_raw, container_raw)) = entry.split_once('=') else {
            continue;
        };
        let host_raw = host_raw.trim();
        let container_raw = container_raw.trim();
        if host_raw.is_empty() || container_raw.is_empty() {
            continue;
        }
        let Some(container) = sanitize_container_prefix(container_raw) else {
            continue;
        };
        let host = normalize_slashes(host_raw);
        if host.is_empty() || host.contains('\0') {
            continue;
        }
        let Some(tail) = match_prefix(&input, &host) else {
            continue;
        };
        if best.as_ref().is_some_and(|(len, _)| host.len() <= *len) {
            continue;
        }
        let Some(mapped) = join_safe(&container, tail) else {
            // A matching host prefix carried a `..` traversal segment: fail closed on this rule.
            return raw.to_string();
        };
        best = Some((host.len(), mapped));
    }

    best.map_or_else(|| raw.to_string(), |(_, mapped)| mapped)
}

fn is_windows_drive(path: &str) -> bool {
    let b = path.as_bytes();
    b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':'
}

fn normalize_slashes(raw: &str) -> String {
    let replaced = raw.replace('\\', "/");
    if replaced == "/" {
        return replaced;
    }
    if is_windows_drive(&replaced) && replaced.len() == 3 && replaced.ends_with(":/") {
        return replaced;
    }
    let trimmed = replaced.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if is_windows_drive(trimmed) && trimmed.len() == 2 && replaced.len() > 2 {
        format!("{trimmed}/")
    } else {
        trimmed.to_string()
    }
}

fn sanitize_container_prefix(raw: &str) -> Option<String> {
    if raw.contains('\0') || raw.contains('\\') {
        return None;
    }
    let norm = normalize_slashes(raw);
    if !norm.starts_with('/') {
        return None;
    }
    if norm.split('/').any(|seg| seg == ".." || seg == ".") {
        return None;
    }
    Some(norm)
}

fn match_prefix<'a>(input: &'a str, host: &str) -> Option<&'a str> {
    if input.len() < host.len() || !input.is_char_boundary(host.len()) {
        return None;
    }
    let prefix = &input[..host.len()];
    let matches = if is_windows_drive(host) {
        prefix.eq_ignore_ascii_case(host)
    } else {
        prefix == host
    };
    if !matches {
        return None;
    }
    let rest = &input[host.len()..];
    if host.ends_with('/') {
        Some(rest)
    } else if rest.is_empty() {
        Some("")
    } else {
        rest.strip_prefix('/')
    }
}

fn join_safe(container: &str, tail: &str) -> Option<String> {
    if tail.contains('\0') {
        return None;
    }
    let mut parts = Vec::new();
    for seg in tail.split('/') {
        if seg == ".." {
            return None;
        }
        if !seg.is_empty() && seg != "." {
            parts.push(seg);
        }
    }
    if parts.is_empty() {
        return Some(container.to_string());
    }
    let joined = parts.join("/");
    if container == "/" {
        Some(format!("/{joined}"))
    } else {
        Some(format!("{container}/{joined}"))
    }
}

#[cfg(test)]
mod tests {
    use super::map_with_spec;

    #[test]
    fn maps_windows_host_path_to_container_prefix() {
        let spec = r"C:\work\repos=/workspaces";
        assert_eq!(
            map_with_spec(r"C:\work\repos\footnote", spec),
            "/workspaces/footnote"
        );
        assert_eq!(
            map_with_spec(r"c:\work\repos\footnote\src\main.rs", spec),
            "/workspaces/footnote/src/main.rs"
        );
        assert_eq!(map_with_spec(r"C:\work\repos\", spec), "/workspaces");
        assert_eq!(map_with_spec("C:/work/repos/app", spec), "/workspaces/app");
    }

    #[test]
    fn enforces_segment_boundaries() {
        let spec = r"C:\work\app=/workspaces";
        assert_eq!(
            map_with_spec(r"C:\work\app2\footnote", spec),
            r"C:\work\app2\footnote"
        );
    }

    #[test]
    fn longest_matching_prefix_wins() {
        let spec = r"C:\work=/host-home;C:\work\repos=/workspaces";
        assert_eq!(
            map_with_spec(r"C:\work\repos\footnote", spec),
            "/workspaces/footnote"
        );
        assert_eq!(
            map_with_spec(r"C:\work\.claude\projects\a.jsonl", spec),
            "/host-home/.claude/projects/a.jsonl"
        );
    }

    #[test]
    fn rejects_parent_traversal_fail_closed() {
        let spec = r"C:\work\repos=/workspaces";
        let hostile = r"C:\work\repos\..\..\Windows\System32";
        assert_eq!(map_with_spec(hostile, spec), hostile);
    }

    #[test]
    fn ignores_malformed_or_relative_container_targets() {
        let spec = r"C:\a=relative/path;C:\b=/workspaces/../etc;C:\c=/valid";
        assert_eq!(map_with_spec(r"C:\a\file", spec), r"C:\a\file");
        assert_eq!(map_with_spec(r"C:\b\file", spec), r"C:\b\file");
        assert_eq!(map_with_spec(r"C:\c\file", spec), "/valid/file");
    }

    #[test]
    fn handles_windows_drive_root_without_matching_drive_relative() {
        let spec = r"D:\=/mnt/d";
        assert_eq!(map_with_spec(r"D:\projects\x", spec), "/mnt/d/projects/x");
        assert_eq!(map_with_spec(r"D:projects\x", spec), r"D:projects\x");
    }

    #[test]
    fn empty_spec_or_already_container_path_is_noop() {
        assert_eq!(
            map_with_spec("/workspaces/footnote", ""),
            "/workspaces/footnote"
        );
        assert_eq!(
            map_with_spec("/workspaces/footnote", r"C:\work\repos=/workspaces"),
            "/workspaces/footnote"
        );
    }
}
