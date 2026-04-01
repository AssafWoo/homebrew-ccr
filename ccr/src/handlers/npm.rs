use super::Handler;

/// Known npm built-in subcommands (not user scripts).
/// If the first argument is NOT in this list it is treated as a script name
/// and `run` is automatically injected before it.
const NPM_BUILTINS: &[&str] = &[
    "access", "adduser", "audit", "bin", "bugs", "cache", "ci", "completion",
    "config", "dedupe", "deprecate", "diff", "dist-tag", "docs", "doctor",
    "edit", "exec", "explain", "explore", "find-dupes", "fund", "get", "help",
    "help-search", "hook", "i", "init", "install", "install-ci-test",
    "install-test", "it", "link", "ll", "login", "logout", "ls", "ls",
    "org", "outdated", "owner", "pack", "ping", "pkg", "prefix", "profile",
    "prune", "publish", "query", "rebuild", "repo", "restart", "root",
    "run", "run-script", "search", "set", "set-script", "shrinkwrap", "star",
    "stars", "start", "stop", "t", "team", "test", "token", "tst", "un",
    "uninstall", "unlink", "unpublish", "unstar", "up", "update", "v",
    "version", "view", "whoami", "add", "remove", "rm", "r",
];

pub struct NpmHandler;

impl Handler for NpmHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        // Auto-inject `run` for user scripts (first arg not a known builtin)
        if !subcmd.is_empty() && !NPM_BUILTINS.contains(&subcmd) {
            let mut out = args.to_vec();
            out.insert(1, "run".to_string());
            return out;
        }
        match subcmd {
            "install" | "i" | "add" | "ci" => {
                if args.iter().any(|a| a == "--no-progress") {
                    args.to_vec()
                } else {
                    let mut out = args.to_vec();
                    out.push("--no-progress".to_string());
                    out
                }
            }
            _ => args.to_vec(),
        }
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "install" | "i" | "add" | "ci" => filter_install(output),
            "test" | "t" => filter_test(output),
            "run" | "run-script" => {
                filter_run_script(output)
            }
            _ => output.to_string(),
        }
    }
}

/// Returns true if a line is npm boilerplate that should be stripped.
fn is_boilerplate_line(line: &str) -> bool {
    let t = line.trim();

    // npm WARN or npm notice lines
    if t.starts_with("npm WARN") || t.starts_with("npm notice") {
        return true;
    }

    // Spinner/progress-only lines: only spaces, dots, /, -, \, |
    if !t.is_empty() && t.chars().all(|c| matches!(c, ' ' | '.' | '/' | '-' | '\\' | '|')) {
        return true;
    }

    // `> project@1.0.0 scriptname` lines (lifecycle script header)
    // Pattern: starts with `> `, then a package name (may contain @, ., /) followed by a space
    // and a script/command word.
    if is_lifecycle_header(t) {
        return true;
    }

    false
}

/// Detect lines like `> package@1.0.0 build` or `> @scope/pkg@2.3.1 start`.
fn is_lifecycle_header(t: &str) -> bool {
    if !t.starts_with("> ") {
        return false;
    }
    let rest = &t[2..];
    // Must have exactly one space separating "pkg@version" from "scriptname"
    // The package part contains at least one '@' or '.' or '/'
    let mut parts = rest.splitn(2, ' ');
    let pkg = parts.next().unwrap_or("");
    let script = parts.next().unwrap_or("").trim();
    if script.is_empty() {
        return false;
    }
    // Package part should look like a name (contains word chars, @, ., /)
    // Script part should be a single word (no spaces)
    let pkg_looks_valid = pkg.chars().any(|c| c == '@' || c == '.' || c == '/') || !pkg.is_empty();
    let script_is_word = !script.contains(' ') && script.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-');
    pkg_looks_valid && script_is_word
}

fn filter_install(output: &str) -> String {
    let mut package_count: Option<u32> = None;
    let mut audit_info: Option<String> = None;

    for line in output.lines() {
        let t = line.trim();

        // Skip boilerplate before examining content
        if is_boilerplate_line(line) {
            continue;
        }

        // npm: "added N packages"
        // pnpm: "N packages added"
        if let Some(n) = extract_package_count(t) {
            package_count = Some(n);
        }
        if t.contains("vulnerabilit") || t.contains("audit") {
            audit_info = Some(t.to_string());
        }
    }

    let count_str = package_count
        .map(|n| format!("{} packages", n))
        .unwrap_or_else(|| "packages".to_string());

    let mut out = format!("[install complete — {}]", count_str);
    if let Some(audit) = audit_info {
        out.push('\n');
        out.push_str(&audit);
    }
    out
}

fn extract_package_count(line: &str) -> Option<u32> {
    // "added 42 packages"
    let words: Vec<&str> = line.split_whitespace().collect();
    for (i, w) in words.iter().enumerate() {
        if (*w == "added" || *w == "installed") && i + 1 < words.len() {
            if let Ok(n) = words[i + 1].parse::<u32>() {
                return Some(n);
            }
        }
    }
    None
}

fn filter_test(output: &str) -> String {
    // Parse test output — keep failures and final summary
    let mut failures: Vec<String> = Vec::new();
    let mut summary_lines: Vec<String> = Vec::new();
    let mut in_failure = false;

    for line in output.lines() {
        let t = line.trim();

        // Jest/vitest failure patterns
        if t.starts_with("✕") || t.starts_with("✗") || t.starts_with("× ") || t.contains("FAIL ") {
            failures.push(t.to_string());
        }

        // Mocha-style "N failing"
        if t.contains("failing") || t.contains("passed") || t.contains("failed") {
            summary_lines.push(t.to_string());
        }

        // Verbose failure output after "●"
        if t.starts_with('●') {
            in_failure = true;
        }
        if in_failure {
            failures.push(t.to_string());
            if t.is_empty() {
                in_failure = false;
            }
        }
    }

    if failures.is_empty() && !summary_lines.is_empty() {
        return summary_lines.join("\n");
    }

    let mut out: Vec<String> = failures;
    if !summary_lines.is_empty() {
        out.push(summary_lines.last().cloned().unwrap_or_default());
    }

    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_run_script(output: &str) -> String {
    // Strip boilerplate and empty lines before processing
    let cleaned: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty() && !is_boilerplate_line(l)
        })
        .collect();

    // If output is short after stripping, pass through
    if cleaned.len() <= 30 {
        return cleaned.join("\n");
    }

    let mut important: Vec<String> = cleaned
        .iter()
        .filter(|l| {
            let lower = l.to_lowercase();
            lower.contains("error")
                || lower.contains("warning")
                || lower.contains("failed")
                || lower.contains("success")
                || lower.contains("done in")
                || lower.contains("built in")
        })
        .map(|l| l.to_string())
        .collect();

    // Always include last 5 lines of cleaned output
    let tail: Vec<String> = cleaned[cleaned.len().saturating_sub(5)..]
        .iter()
        .map(|l| l.to_string())
        .collect();

    important.push(format!("[{} lines of output]", cleaned.len()));
    important.extend(tail);
    important.dedup();
    important.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::handlers::Handler;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rewrite_args_injects_run_for_unknown_subcommand() {
        let handler = NpmHandler;
        // "build" is not a builtin — should become "npm run build"
        let result = handler.rewrite_args(&args(&["npm", "build"]));
        assert_eq!(result[1], "run", "should inject 'run'");
        assert_eq!(result[2], "build", "script name should remain");
    }

    #[test]
    fn rewrite_args_does_not_inject_run_for_builtins() {
        let handler = NpmHandler;
        let result = handler.rewrite_args(&args(&["npm", "install", "lodash"]));
        assert_eq!(result[1], "install", "'install' is a builtin — no run injection");
        let result2 = handler.rewrite_args(&args(&["npm", "test"]));
        assert_eq!(result2[1], "test", "'test' is a builtin — no run injection");
    }

    #[test]
    fn npm_warn_lines_dropped_from_install_output() {
        let handler = NpmHandler;
        let output = "\
npm WARN deprecated lodash@3.0.0: use lodash@4 instead
npm notice created a lockfile
added 42 packages from 30 contributors
npm WARN optional SKIPPING OPTIONAL DEPENDENCY";
        let result = handler.filter(output, &args(&["npm", "install"]));
        assert!(!result.contains("npm WARN"), "npm WARN lines should be stripped");
        assert!(!result.contains("npm notice"), "npm notice lines should be stripped");
        assert!(result.contains("42 packages"), "package count should be kept");
    }

    #[test]
    fn lifecycle_header_lines_dropped_from_install() {
        let handler = NpmHandler;
        let output = "\
> my-project@1.0.0 prepare
> husky install

added 10 packages";
        let result = handler.filter(output, &args(&["npm", "install"]));
        assert!(!result.contains("> my-project@1.0.0 prepare"), "lifecycle header should be stripped");
        assert!(result.contains("10 packages"), "package count should be kept");
    }

    #[test]
    fn package_count_summary_kept() {
        let handler = NpmHandler;
        let output = "\
npm WARN deprecated foo@1.0.0: bar
> project@0.1.0 postinstall
added 123 packages in 4.2s";
        let result = handler.filter(output, &args(&["npm", "install"]));
        assert!(result.contains("123 packages"), "package count summary must be present");
        assert!(!result.contains("npm WARN"), "WARN lines must be stripped");
    }

    #[test]
    fn run_script_strips_boilerplate_and_empty_lines() {
        // Build output with > lifecycle header and empty lines mixed in
        let mut lines: Vec<String> = vec![
            "> my-app@1.0.0 build".to_string(),
            String::new(),
        ];
        // Add 35 real output lines
        for i in 1..=35 {
            lines.push(format!("Building module {}", i));
        }
        lines.push("Build complete in 5s".to_string());
        let output = lines.join("\n");
        let result = filter_run_script(&output);
        // Should not contain the lifecycle header
        assert!(!result.contains("> my-app@1.0.0 build"), "lifecycle header should be stripped");
        // Should mention the success line
        assert!(result.contains("Build complete"), "important lines should be kept");
    }
}
