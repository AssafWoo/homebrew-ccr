use super::Handler;

pub struct EslintHandler;

impl Handler for EslintHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();

        // Check for clean run
        if output.trim().is_empty()
            || lines
                .iter()
                .all(|l| l.trim().is_empty() || l.contains("0 problems"))
        {
            return "All files clean".to_string();
        }

        let mut out: Vec<String> = Vec::new();
        let mut total_errors = 0usize;
        let mut error_lines: Vec<String> = Vec::new();

        // file path headers are lines that look like absolute/relative paths (not indented)
        // error/warning lines are indented with line/col info
        let mut current_file: Option<String> = None;
        let mut file_has_errors = false;

        for line in &lines {
            let t = line.trim();
            if t.is_empty() {
                if file_has_errors {
                    if let Some(ref f) = current_file {
                        out.push(f.clone());
                        out.extend(error_lines.drain(..));
                    }
                }
                current_file = None;
                file_has_errors = false;
                error_lines.clear();
                continue;
            }
            // Global summary line: "✖ N problems (M errors, P warnings)"
            if t.starts_with('✖') || t.starts_with("✖") || t.contains(" problems") {
                // Only keep the global total (not per-file ✖ N problems)
                if !t.contains("  ") {
                    out.push(line.to_string());
                }
                continue;
            }
            // Per-file ✖ N problems line — drop it
            if t.starts_with("✖") {
                continue;
            }
            // File path line (not indented, contains path separators)
            if !line.starts_with(' ') && (t.contains('/') || t.contains('\\') || t.ends_with(".js") || t.ends_with(".ts") || t.ends_with(".tsx") || t.ends_with(".jsx")) {
                // Flush previous
                if file_has_errors {
                    if let Some(ref f) = current_file {
                        out.push(f.clone());
                        out.extend(error_lines.drain(..));
                    }
                }
                current_file = Some(line.to_string());
                file_has_errors = false;
                error_lines.clear();
                continue;
            }
            // Error/warning line (indented): "  42:5  error  'foo' is not defined  no-undef"
            if line.starts_with(' ') && (t.contains("error") || t.contains("warning")) {
                total_errors += 1;
                if total_errors <= 30 {
                    error_lines.push(line.to_string());
                    file_has_errors = true;
                }
            }
        }
        // Flush last file
        if file_has_errors {
            if let Some(ref f) = current_file {
                out.push(f.clone());
                out.extend(error_lines.drain(..));
            }
        }

        if total_errors > 30 {
            out.push(format!("[+{} more errors]", total_errors - 20));
        }

        if out.is_empty() {
            output.to_string()
        } else {
            out.join("\n")
        }
    }
}
