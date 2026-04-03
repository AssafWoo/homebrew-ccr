use anyhow::Result;

/// Rewrite a command string for PreToolUse injection.
/// Prints the rewritten command and exits 0, or exits 1 if no rewrite is needed.
pub fn run(command: String) -> Result<()> {
    let rewritten = rewrite_command(&command);
    match rewritten {
        Some(r) => {
            print!("{}", r);
            Ok(())
        }
        None => {
            // No rewrite — exit 1 so the hook passes through silently
            std::process::exit(1);
        }
    }
}

/// Rewrite a full command string. Returns `Some(rewritten)` if rewrite is needed,
/// or `None` if no handler matches or already wrapped.
pub fn rewrite_command(command: &str) -> Option<String> {
    // Handle compound commands: &&, ||, ;
    // Try to split and rewrite each part
    if let Some(result) = rewrite_compound(command, " && ") {
        return Some(result);
    }
    if let Some(result) = rewrite_compound(command, " || ") {
        return Some(result);
    }
    if let Some(result) = rewrite_compound(command, "; ") {
        return Some(result);
    }

    // Single command
    rewrite_single(command)
}

/// Returns true if `command` contains a stdout redirect (`>`, `>>`) or pipe
/// (`|`) outside of single or double quotes.
///
/// Simple heuristic — not a full shell parser. Commands whose stdout is
/// diverted to a file or another process must not be wrapped with `ccr run`,
/// because CCR's dedup/delta annotations would replace the real content.
fn has_stdout_diversion(command: &str) -> bool {
    let mut in_single = false;
    let mut in_double = false;
    let bytes = command.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\'' if !in_double => { in_single = !in_single; }
            b'"'  if !in_single => { in_double = !in_double; }
            b'>'  if !in_single && !in_double => {
                // Exclude '->' and '=>' (not a redirect)
                let prev = if i > 0 { bytes[i - 1] } else { b' ' };
                if prev != b'-' && prev != b'=' {
                    return true;
                }
            }
            b'|'  if !in_single && !in_double => {
                // '||' is logical OR, not a pipe — skip both characters
                if i + 1 < bytes.len() && bytes[i + 1] == b'|' {
                    i += 1; // skip the second '|'
                } else {
                    return true;
                }
            }
            _ => {}
        }
        i += 1;
    }
    false
}

/// Rewrite a single (non-compound) command.
/// Uses the handler's `rewrite_args` to inject flags (e.g. --message-format json)
/// so the rewritten command string reflects the actual args that will be run.
fn rewrite_single(command: &str) -> Option<String> {
    let trimmed = command.trim();

    // Don't double-wrap
    if trimmed.starts_with("ccr run ") || trimmed == "ccr run" {
        return None;
    }

    // Never wrap commands that divert stdout (redirect or pipe).
    // CCR's dedup/delta annotations would replace real content.
    if has_stdout_diversion(trimmed) {
        return None;
    }

    // Extract argv[0]
    let first = trimmed.split_whitespace().next()?;

    let handler = crate::handlers::get_handler(first)?;

    // Build the flag-injected arg list via the handler
    let args: Vec<String> = trimmed.split_whitespace().map(String::from).collect();
    let rewritten_args = handler.rewrite_args(&args);

    Some(format!("ccr run {}", rewritten_args.join(" ")))
}

/// Try to split a compound command on `operator` and rewrite each part.
/// Returns `Some(rewritten)` only if at least one part was rewritten.
fn rewrite_compound(command: &str, operator: &str) -> Option<String> {
    if !command.contains(operator) {
        return None;
    }

    let parts: Vec<&str> = command.split(operator).collect();
    if parts.len() < 2 {
        return None;
    }

    let mut any_rewritten = false;
    let rewritten: Vec<String> = parts
        .iter()
        .map(|part| {
            if let Some(r) = rewrite_single(part.trim()) {
                any_rewritten = true;
                r
            } else {
                part.trim().to_string()
            }
        })
        .collect();

    if any_rewritten {
        Some(rewritten.join(operator))
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_command_rewritten() {
        let result = rewrite_command("git status");
        // git status gets --porcelain injected via rewrite_args
        assert_eq!(result, Some("ccr run git status --porcelain".to_string()));
    }

    #[test]
    fn flag_injection_for_cargo_build() {
        let result = rewrite_command("cargo build");
        // cargo build gets --message-format json injected
        let r = result.expect("cargo build should be rewritten");
        assert!(r.starts_with("ccr run cargo build"), "should be wrapped: {}", r);
        assert!(r.contains("--message-format"), "should inject --message-format: {}", r);
        assert!(r.contains("json"), "should inject json format: {}", r);
    }

    #[test]
    fn no_double_flag_injection() {
        // If --message-format already present, it should not be added again
        let result = rewrite_command("cargo build --message-format human");
        let r = result.expect("should be rewritten");
        let count = r.matches("--message-format").count();
        assert_eq!(count, 1, "flag should appear exactly once: {}", r);
    }

    #[test]
    fn unknown_command_not_rewritten() {
        let result = rewrite_command("some-unknown-tool --flag");
        assert_eq!(result, None);
    }

    #[test]
    fn no_double_wrap() {
        let result = rewrite_command("ccr run git status");
        assert_eq!(result, None);
    }

    #[test]
    fn compound_and() {
        let result = rewrite_command("cargo build && git push");
        let r = result.expect("should be rewritten");
        assert!(r.contains("ccr run cargo build"), "cargo part: {}", r);
        assert!(r.contains("ccr run git push"), "git part: {}", r);
        assert!(r.contains(" && "), "should preserve && operator: {}", r);
    }

    #[test]
    fn compound_mixed() {
        // Only known commands get wrapped; git status gets --porcelain injected
        let result = rewrite_command("some-tool && git status");
        assert_eq!(result, Some("some-tool && ccr run git status --porcelain".to_string()));
    }

    #[test]
    fn compound_no_known() {
        // No known commands → no rewrite
        let result = rewrite_command("tool-a && tool-b");
        assert_eq!(result, None);
    }

    #[test]
    fn redirect_bare() {
        assert!(has_stdout_diversion("git show HEAD:src/main.rs > main.rs"));
    }

    #[test]
    fn redirect_append() {
        assert!(has_stdout_diversion("cargo build >> build.log"));
    }

    #[test]
    fn redirect_inside_single_quotes_not_detected() {
        // > inside quotes is not a redirect
        assert!(!has_stdout_diversion("echo 'a > b'"));
    }

    #[test]
    fn redirect_inside_double_quotes_not_detected() {
        assert!(!has_stdout_diversion("echo \"a > b\""));
    }

    #[test]
    fn arrow_operators_not_redirect() {
        // -> and => in code snippets / descriptions must not trigger
        assert!(!has_stdout_diversion("git log --format='%H -> %s'"));
        assert!(!has_stdout_diversion("some-tool => output"));
    }

    #[test]
    fn pipe_detected() {
        assert!(has_stdout_diversion("git log | head -5"));
    }

    #[test]
    fn pipe_inside_quotes_not_detected() {
        assert!(!has_stdout_diversion("echo 'a | b'"));
        assert!(!has_stdout_diversion("echo \"a | b\""));
    }

    #[test]
    fn logical_or_not_detected_as_pipe() {
        assert!(!has_stdout_diversion("test -f foo || echo missing"));
    }

    #[test]
    fn pipe_with_redirect_detected() {
        assert!(has_stdout_diversion("git show HEAD:file | head -1"));
    }

    #[test]
    fn piped_command_not_wrapped() {
        let result = rewrite_command("git show HEAD:file | head -5");
        assert_eq!(result, None, "should not wrap a piped command");
    }

    #[test]
    fn subshell_pipe_detected_as_false_positive() {
        // Pipes inside $() are detected — accepted trade-off since we are
        // not a full shell parser. Prevents wrapping, which is the safe default.
        assert!(has_stdout_diversion("echo $(git log | head -1)"));
    }

    #[test]
    fn pipe_at_start_of_string() {
        assert!(has_stdout_diversion("| cat"));
    }

    #[test]
    fn compound_with_pipe_only_wraps_non_piped_part() {
        // rewrite_compound splits on && then has_stdout_diversion guards each part.
        // The piped part must NOT be wrapped; the non-piped part should be.
        let result = rewrite_command("cargo build && git log | head -5");
        assert!(result.is_some(), "compound should still rewrite the non-piped part");
        let r = result.unwrap();
        assert!(r.contains("ccr run cargo build"), "cargo build should be wrapped");
        assert!(!r.contains("ccr run git log"), "piped git log must not be wrapped");
        assert!(r.contains("git log | head -5"), "piped part should pass through unchanged");
    }

    #[test]
    fn git_show_redirect_not_wrapped() {
        // git show with redirect must not be wrapped — would corrupt the output file
        let result = rewrite_command("git show origin/main:src/lib.rs > /tmp/lib.rs");
        assert_eq!(result, None, "should not wrap a redirected command");
    }

    #[test]
    fn git_show_no_redirect_still_wrapped() {
        // git show without redirect should still be wrapped normally
        let result = rewrite_command("git show HEAD");
        assert!(result.is_some(), "should wrap git show without redirect");
        assert!(result.unwrap().starts_with("ccr run git show"));
    }
}
