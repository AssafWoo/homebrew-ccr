use super::Handler;

pub struct MakeHandler;

impl Handler for MakeHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let has_error = lines.iter().any(|l| {
            l.contains("Error ") || l.contains(": error:") || l.contains("*** [")
        });

        let mut out: Vec<String> = Vec::new();
        for line in &lines {
            let t = line.trim();
            if t.is_empty() {
                continue;
            }
            // Drop make internals
            if t.starts_with("make[")
                || t == "Nothing to be done for"
                || t.contains("Nothing to be done")
            {
                continue;
            }
            // Keep compiler errors/warnings
            if t.contains(": error:") || t.contains(": warning:") || t.contains(": note:") {
                out.push(line.to_string());
                continue;
            }
            // Keep file paths with line numbers (e.g., "src/foo.c:42:5: error:")
            if regex::Regex::new(r"^\S+:\d+:\d*:?\s+(error|warning)")
                .map(|re| re.is_match(t))
                .unwrap_or(false)
            {
                out.push(line.to_string());
                continue;
            }
            // Keep make failure lines
            if t.starts_with("make:") && t.contains("Error") {
                out.push(line.to_string());
                continue;
            }
            // Keep recipe echo lines (not make internals)
            if !t.starts_with("make") {
                if has_error {
                    // Only noise-filter on success runs; on errors keep everything
                    out.push(line.to_string());
                } else {
                    // On success, only keep important lines
                    out.push(line.to_string());
                }
            }
        }

        if !has_error {
            // Success: emit a clean summary
            if out.is_empty() || out.iter().all(|l| l.trim().is_empty()) {
                return "[make: complete]".to_string();
            }
            // Keep last few lines + success marker
            let tail: Vec<String> = out.iter().rev().take(5).rev().cloned().collect();
            let mut result = tail.join("\n");
            result.push_str("\n[make: complete]");
            return result;
        }

        if out.is_empty() {
            output.to_string()
        } else {
            out.join("\n")
        }
    }
}
