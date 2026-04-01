use super::Handler;

pub struct RspecHandler;

impl Handler for RspecHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        // Inject --format json if no format flag is already present
        if !args.iter().any(|a| a == "--format" || a == "-f") {
            let mut out = args.to_vec();
            out.push("--format".to_string());
            out.push("json".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, _args: &[String]) -> String {
        // RSpec JSON output may be preceded by progress dots/text; find the JSON object.
        let trimmed = output.trim();
        // Walk from the end looking for the outermost JSON object
        if let Some(json_start) = trimmed.rfind('{') {
            let json_candidate = &trimmed[json_start..];
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_candidate) {
                if let Some(result) = parse_rspec_json(&v) {
                    return result;
                }
            }
        }
        // Fallback: text-based filtering
        filter_rspec_text(output)
    }
}

fn parse_rspec_json(v: &serde_json::Value) -> Option<String> {
    let summary  = v.get("summary")?;
    let examples = v.get("examples")?.as_array()?;

    let total    = summary.get("example_count").and_then(|n| n.as_u64()).unwrap_or(0);
    let failed   = summary.get("failure_count").and_then(|n| n.as_u64()).unwrap_or(0);
    let pending  = summary.get("pending_count").and_then(|n| n.as_u64()).unwrap_or(0);
    let duration = summary.get("duration").and_then(|d| d.as_f64()).unwrap_or(0.0);

    let mut out: Vec<String> = Vec::new();

    if failed == 0 {
        let mut line = format!("[{} examples, 0 failures ({:.2}s)]", total, duration);
        if pending > 0 {
            line.push_str(&format!(", {} pending", pending));
        }
        out.push(line);
        return Some(out.join("\n"));
    }

    // Emit each failing example with its message and location
    for ex in examples {
        let status = ex.get("status").and_then(|s| s.as_str()).unwrap_or("");
        if status != "failed" {
            continue;
        }
        let desc = ex.get("full_description").and_then(|d| d.as_str()).unwrap_or("?");
        let loc  = ex.get("location").and_then(|l| l.as_str()).unwrap_or("");
        out.push(format!("FAIL: {}", desc));

        if let Some(exc) = ex.get("exception") {
            let msg = exc.get("message").and_then(|m| m.as_str()).unwrap_or("");
            for (i, line) in msg.lines().enumerate() {
                if i >= 5 {
                    out.push("  [... truncated ...]".to_string());
                    break;
                }
                out.push(format!("  {}", line));
            }
        }
        if !loc.is_empty() {
            out.push(format!("  at {}", loc));
        }
    }

    out.push(format!(
        "[{} examples, {} failures ({:.2}s)]",
        total, failed, duration
    ));
    Some(out.join("\n"))
}

fn filter_rspec_text(output: &str) -> String {
    let mut important: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0usize;

    for line in output.lines() {
        let t = line.trim();

        if t.starts_with("Failure/Error:") || (t.starts_with("rspec ") && t.contains(":")) {
            in_failure = true;
            failure_lines = 0;
        }

        if in_failure {
            important.push(line.to_string());
            failure_lines += 1;
            if (t.is_empty() && failure_lines > 2) || failure_lines >= 10 {
                in_failure = false;
            }
            continue;
        }

        // Summary line: "N examples, N failures"
        if t.contains("example") && (t.contains("failure") || t.contains("pending")) {
            important.push(line.to_string());
        }
    }

    if important.is_empty() {
        output.to_string()
    } else {
        important.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn rewrite_args_injects_format_json() {
        let handler = RspecHandler;
        let out = handler.rewrite_args(&args(&["rspec", "spec/"]));
        assert!(out.contains(&"--format".to_string()));
        assert!(out.contains(&"json".to_string()));
    }

    #[test]
    fn rewrite_args_does_not_duplicate_format() {
        let handler = RspecHandler;
        let out = handler.rewrite_args(&args(&["rspec", "--format", "progress", "spec/"]));
        let count = out.iter().filter(|a| a.as_str() == "--format").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn parse_rspec_json_all_pass() {
        let json = serde_json::json!({
            "examples": [
                { "status": "passed", "full_description": "Foo works" }
            ],
            "summary": {
                "example_count": 1,
                "failure_count": 0,
                "pending_count": 0,
                "duration": 0.42
            }
        });
        let result = parse_rspec_json(&json).unwrap();
        assert!(result.contains("1 examples, 0 failures"), "got: {}", result);
    }

    #[test]
    fn parse_rspec_json_with_failure() {
        let json = serde_json::json!({
            "examples": [
                {
                    "status": "failed",
                    "full_description": "Foo fails",
                    "location": "spec/foo_spec.rb:10",
                    "exception": { "message": "expected 1 got 2" }
                }
            ],
            "summary": {
                "example_count": 1,
                "failure_count": 1,
                "pending_count": 0,
                "duration": 0.1
            }
        });
        let result = parse_rspec_json(&json).unwrap();
        assert!(result.contains("FAIL: Foo fails"), "got: {}", result);
        assert!(result.contains("expected 1 got 2"), "got: {}", result);
        assert!(result.contains("1 examples, 1 failures"), "got: {}", result);
    }
}
