use super::Handler;

pub struct TscHandler;

impl Handler for TscHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        // Clean build
        if output.contains("Found 0 errors") {
            return "Build OK".to_string();
        }

        let lines: Vec<&str> = output.lines().collect();
        let mut error_count = 0usize;
        let mut warning_count = 0usize;

        // Group errors/warnings by file
        // Lines like: src/foo.ts(42,5): error TS2345: ...
        let mut grouped: Vec<(String, Vec<String>)> = Vec::new(); // (file, messages)
        let ts_re = regex::Regex::new(r"^(.+\.tsx?)\((\d+),\d+\):\s+(error|warning)\s+(TS\d+:.+)$")
            .unwrap();

        for line in &lines {
            if let Some(caps) = ts_re.captures(line) {
                let file = caps[1].to_string();
                let lineno = &caps[2];
                let kind = &caps[3];
                let msg = &caps[4];

                if kind == "error" {
                    error_count += 1;
                } else {
                    warning_count += 1;
                }

                let entry = format!("  L{}: {} {}", lineno, kind, msg);
                if let Some(last) = grouped.last_mut() {
                    if last.0 == file {
                        last.1.push(entry);
                        continue;
                    }
                }
                grouped.push((file, vec![entry]));
            }
        }

        if grouped.is_empty() {
            return output.to_string();
        }

        let mut out: Vec<String> = Vec::new();
        for (file, messages) in &grouped {
            out.push(file.clone());
            out.extend(messages.iter().cloned());
        }
        out.push(format!("[{} errors, {} warnings]", error_count, warning_count));
        out.join("\n")
    }
}
