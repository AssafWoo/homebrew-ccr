use super::Handler;

pub struct CargoHandler;

impl Handler for CargoHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "build" | "check" | "clippy" => {
                // Inject --message-format json unless already present
                if args.iter().any(|a| a.starts_with("--message-format")) {
                    args.to_vec()
                } else {
                    let mut out = args.to_vec();
                    out.push("--message-format".to_string());
                    out.push("json".to_string());
                    out
                }
            }
            _ => args.to_vec(),
        }
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "build" | "check" | "clippy" => filter_build(output),
            "test" | "nextest" => filter_test(output),
            _ => output.to_string(),
        }
    }
}

/// Filter `cargo build/check/clippy --message-format json` output.
/// Keeps only compiler-message (errors + warnings); discards compiler-artifact noise.
fn filter_build(output: &str) -> String {
    let mut errors: Vec<String> = Vec::new();
    let mut warnings: Vec<String> = Vec::new();
    let mut success: Option<bool> = None;

    for line in output.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        // Try JSON parse first
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
            match v.get("reason").and_then(|r| r.as_str()) {
                Some("compiler-message") => {
                    if let Some(msg) = v.get("message") {
                        let level = msg.get("level").and_then(|l| l.as_str()).unwrap_or("");
                        let text = msg.get("message").and_then(|m| m.as_str()).unwrap_or("");
                        let location = msg
                            .get("spans")
                            .and_then(|s| s.as_array())
                            .and_then(|s| s.first())
                            .map(|span| {
                                let file = span
                                    .get("file_name")
                                    .and_then(|f| f.as_str())
                                    .unwrap_or("");
                                let line_n =
                                    span.get("line_start").and_then(|l| l.as_u64()).unwrap_or(0);
                                format!(" [{}:{}]", file, line_n)
                            })
                            .unwrap_or_default();

                        match level {
                            "error" | "error[E]" => {
                                errors.push(format!("error: {}{}", text, location));
                            }
                            "warning" => {
                                warnings.push(format!("warning: {}{}", text, location));
                            }
                            _ => {}
                        }
                    }
                }
                Some("build-finished") => {
                    success = v.get("success").and_then(|s| s.as_bool());
                }
                _ => {}
            }
        } else {
            // Non-JSON line (e.g. cargo stderr without JSON flag, or mixed output)
            // Keep error/warning lines
            if trimmed.starts_with("error") || trimmed.starts_with("warning") {
                if trimmed.starts_with("error") {
                    errors.push(trimmed.to_string());
                } else {
                    warnings.push(trimmed.to_string());
                }
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    out.extend(errors.iter().cloned());
    if !warnings.is_empty() {
        out.push(format!("[{} warnings]", warnings.len()));
        // Show first 3 warnings
        for w in warnings.iter().take(3) {
            out.push(format!("  {}", w));
        }
        if warnings.len() > 3 {
            out.push(format!("  [+{} more warnings]", warnings.len() - 3));
        }
    }
    match success {
        Some(true) => {
            if out.is_empty() {
                out.push("Build OK".to_string());
            }
        }
        Some(false) => {
            if errors.is_empty() {
                out.push("Build FAILED".to_string());
            }
        }
        None => {}
    }

    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

/// Filter `cargo test` standard output.
/// Keeps failures, the final summary line, and failure detail sections.
fn filter_test(output: &str) -> String {
    let mut failures: Vec<String> = Vec::new();
    let mut summary: Option<String> = None;
    let mut in_failure_detail = false;
    let mut failure_detail: Vec<String> = Vec::new();
    let mut failure_names: Vec<String> = Vec::new();

    for line in output.lines() {
        // Detect failure test lines: "test some::path ... FAILED"
        if line.trim_start().starts_with("test ") && line.ends_with("FAILED") {
            let name = line.trim_start()
                .trim_start_matches("test ")
                .trim_end_matches(" ... FAILED")
                .to_string();
            failure_names.push(name);
        }

        // Final result line
        if line.starts_with("test result:") {
            summary = Some(line.to_string());
        }

        // Failure detail sections
        if line.starts_with("failures:") {
            in_failure_detail = true;
        }
        if in_failure_detail {
            failure_detail.push(line.to_string());
        }
    }

    // If all passed
    if failure_names.is_empty() {
        if let Some(s) = summary {
            // Count from summary line
            return s;
        }
        return output.to_string();
    }

    // Build compact output
    let mut out: Vec<String> = Vec::new();
    for name in &failure_names {
        failures.push(format!("FAILED: {}", name));
    }
    out.extend(failures);

    // Add failure details (truncated)
    if !failure_detail.is_empty() {
        let detail_lines: Vec<&str> = failure_detail
            .iter()
            .map(|s| s.as_str())
            .filter(|l| {
                !l.trim().is_empty()
                    && !l.starts_with("failures:")
                    && !l.starts_with("test result:")
            })
            .take(20)
            .collect();
        out.push(String::new());
        out.extend(detail_lines.iter().map(|l| l.to_string()));
    }

    if let Some(s) = summary {
        out.push(s);
    }

    out.join("\n")
}
