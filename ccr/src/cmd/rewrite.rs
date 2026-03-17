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

/// Rewrite a single (non-compound) command.
fn rewrite_single(command: &str) -> Option<String> {
    let trimmed = command.trim();

    // Don't double-wrap
    if trimmed.starts_with("ccr run ") || trimmed == "ccr run" {
        return None;
    }

    // Extract argv[0]
    let first = trimmed.split_whitespace().next()?;

    if crate::handlers::is_known(first) {
        Some(format!("ccr run {}", trimmed))
    } else {
        None
    }
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
        assert_eq!(result, Some("ccr run git status".to_string()));
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
        assert_eq!(
            result,
            Some("ccr run cargo build && ccr run git push".to_string())
        );
    }

    #[test]
    fn compound_mixed() {
        // Only known commands get wrapped
        let result = rewrite_command("some-tool && git status");
        assert_eq!(result, Some("some-tool && ccr run git status".to_string()));
    }

    #[test]
    fn compound_no_known() {
        // No known commands → no rewrite
        let result = rewrite_command("tool-a && tool-b");
        assert_eq!(result, None);
    }
}
