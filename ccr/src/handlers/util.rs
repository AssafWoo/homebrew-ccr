/// Parse a space-aligned table, keep only specified column indices (0-based).
pub fn compact_table(output: &str, keep_cols: &[usize]) -> String {
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut out: Vec<String> = Vec::new();
    for line in &lines {
        if line.trim().is_empty() {
            continue;
        }
        let cols: Vec<&str> = line.split_whitespace().collect();
        if cols.is_empty() {
            continue;
        }
        let selected: Vec<&str> = keep_cols
            .iter()
            .filter_map(|&i| cols.get(i).copied())
            .collect();
        out.push(selected.join("  "));
    }
    out.join("\n")
}

/// Extract failure blocks + summary from test runner output.
/// runner: "pytest" | "jest" | "vitest" | "dotnet"
pub fn test_failures(output: &str, runner: &str) -> String {
    match runner {
        "pytest" => filter_pytest(output),
        "jest" => filter_jest(output),
        "vitest" => filter_vitest(output),
        _ => output.to_string(),
    }
}

fn filter_pytest(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0;

    for line in &lines {
        let t = line.trim();
        // Keep FAILED/ERROR node IDs
        if t.starts_with("FAILED ") || t.starts_with("ERROR ") {
            out.push(line.to_string());
            continue;
        }
        // Start of a failure block
        if t.starts_with("____") && t.ends_with("____") {
            in_failure = true;
            failure_lines = 0;
            out.push(line.to_string());
            continue;
        }
        if in_failure {
            if failure_lines < 10 {
                out.push(line.to_string());
                failure_lines += 1;
            } else if failure_lines == 10 {
                out.push("[... truncated ...]".to_string());
                failure_lines += 1;
            }
            // End of failure block
            if t.starts_with("====") {
                in_failure = false;
            }
            continue;
        }
        // Summary line
        if t.contains(" failed") || t.contains(" passed") || t.contains(" error") {
            if t.starts_with('=') {
                out.push(line.to_string());
            }
        }
        // Drop: PASSED lines, dots, "collected N items", platform header
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_jest(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0;

    for line in &lines {
        let t = line.trim();
        // Keep FAIL <path> lines
        if t.starts_with("FAIL ") {
            out.push(line.to_string());
            in_failure = false;
            continue;
        }
        // Keep ● failure detail blocks
        if t.starts_with('●') {
            in_failure = true;
            failure_lines = 0;
            out.push(line.to_string());
            continue;
        }
        if in_failure {
            if failure_lines < 15 {
                out.push(line.to_string());
                failure_lines += 1;
            } else if failure_lines == 15 {
                out.push("[... truncated ...]".to_string());
                failure_lines += 1;
            }
            // Blank line ends the block
            if t.is_empty() && failure_lines > 2 {
                in_failure = false;
            }
            continue;
        }
        // Final summary
        if t.starts_with("Tests:") || t.starts_with("Test Suites:") || t.starts_with("Time:") {
            out.push(line.to_string());
            continue;
        }
        // Drop: PASS lines, ✓ lines, -- separators
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_vitest(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0;

    for line in &lines {
        let t = line.trim();
        // Keep FAIL lines
        if t.starts_with("FAIL") && t.contains(' ') {
            out.push(line.to_string());
            in_failure = false;
            continue;
        }
        // Error message lines
        if t.starts_with("× ") || t.starts_with("✗ ") {
            in_failure = true;
            failure_lines = 0;
            out.push(line.to_string());
            continue;
        }
        if in_failure {
            if failure_lines < 5 {
                out.push(line.to_string());
                failure_lines += 1;
            }
            if t.is_empty() && failure_lines > 1 {
                in_failure = false;
            }
            continue;
        }
        // Summary line
        if t.starts_with("Tests") && (t.contains("failed") || t.contains("passed")) {
            out.push(line.to_string());
        }
        // Drop: ✓ passing lines, progress bars, module noise
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

/// Cosine similarity between two float vectors.
pub fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}
