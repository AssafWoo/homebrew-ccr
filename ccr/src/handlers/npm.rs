use super::Handler;

pub struct NpmHandler;

impl Handler for NpmHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "install" | "i" | "add" | "ci" => filter_install(output),
            "test" | "t" => filter_test(output),
            "run" => {
                // For npm run <script>, filter based on output content
                filter_run_script(output)
            }
            _ => output.to_string(),
        }
    }
}

fn filter_install(output: &str) -> String {
    let mut package_count: Option<u32> = None;
    let mut audit_info: Option<String> = None;

    for line in output.lines() {
        let t = line.trim();
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
    // For build scripts: keep errors/warnings + last N lines of output
    let lines: Vec<&str> = output.lines().collect();

    // If output is short, pass through
    if lines.len() <= 30 {
        return output.to_string();
    }

    let mut important: Vec<String> = lines
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

    // Always include last 5 lines
    let tail: Vec<String> = lines[lines.len().saturating_sub(5)..]
        .iter()
        .map(|l| l.to_string())
        .collect();

    important.push(format!("[{} lines of output]", lines.len()));
    important.extend(tail);
    important.dedup();
    important.join("\n")
}
