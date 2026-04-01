use super::Handler;

pub struct RubocopHandler;

impl Handler for RubocopHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        if !args.iter().any(|a| a == "--format" || a == "-f") {
            let mut out = args.to_vec();
            out.push("--format".to_string());
            out.push("json".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, _args: &[String]) -> String {
        let trimmed = output.trim();
        if trimmed.starts_with('{') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                if let Some(result) = parse_rubocop_json(&v) {
                    return result;
                }
            }
        }
        filter_rubocop_text(output)
    }
}

const MAX_PER_SEVERITY: usize = 10;
const MAX_CONVENTIONS: usize = 5;

fn parse_rubocop_json(v: &serde_json::Value) -> Option<String> {
    let files   = v.get("files")?.as_array()?;
    let summary = v.get("summary")?;

    let offense_count = summary.get("offense_count").and_then(|n| n.as_u64()).unwrap_or(0);
    let file_count    = summary.get("inspected_file_count").and_then(|n| n.as_u64()).unwrap_or(0);

    if offense_count == 0 {
        return Some(format!(
            "[rubocop: {} file(s) inspected, no offenses]",
            file_count
        ));
    }

    let mut errors:      Vec<String> = Vec::new();
    let mut warnings:    Vec<String> = Vec::new();
    let mut conventions: Vec<String> = Vec::new();

    for file in files {
        let path = file.get("path").and_then(|p| p.as_str()).unwrap_or("?");
        let offenses = match file.get("offenses").and_then(|o| o.as_array()) {
            Some(o) => o,
            None    => continue,
        };
        for offense in offenses {
            let severity = offense.get("severity").and_then(|s| s.as_str()).unwrap_or("convention");
            let msg      = offense.get("message").and_then(|m| m.as_str()).unwrap_or("");
            let cop      = offense.get("cop_name").and_then(|c| c.as_str()).unwrap_or("");
            let line_no  = offense
                .get("location")
                .and_then(|l| l.get("line"))
                .and_then(|l| l.as_u64())
                .unwrap_or(0);
            let entry = format!("{}:{}: [{}] {} ({})", path, line_no, severity, msg, cop);
            match severity {
                "error" | "fatal" => errors.push(entry),
                "warning" | "refactor" => warnings.push(entry),
                _ => conventions.push(entry),
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    for e in errors.iter().take(MAX_PER_SEVERITY)   { out.push(e.clone()); }
    for w in warnings.iter().take(MAX_PER_SEVERITY) { out.push(w.clone()); }

    let conv_shown = conventions.len().min(MAX_CONVENTIONS);
    for c in conventions.iter().take(conv_shown) { out.push(c.clone()); }
    if conventions.len() > conv_shown {
        out.push(format!(
            "[+{} more convention/style offenses]",
            conventions.len() - conv_shown
        ));
    }

    out.push(format!(
        "[rubocop: {} file(s) inspected, {} offense(s)]",
        file_count, offense_count
    ));
    Some(out.join("\n"))
}

fn filter_rubocop_text(output: &str) -> String {
    let mut important: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        // Offense lines contain severity letter after file:line:col:
        if t.contains(": C: ")
            || t.contains(": E: ")
            || t.contains(": W: ")
            || t.contains(": F: ")
            || t.contains(": R: ")
        {
            important.push(line.to_string());
        }
        // Summary
        if t.contains("offense") && (t.contains("file") || t.contains("inspected")) {
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
        let handler = RubocopHandler;
        let out = handler.rewrite_args(&args(&["rubocop", "lib/"]));
        assert!(out.contains(&"--format".to_string()));
        assert!(out.contains(&"json".to_string()));
    }

    #[test]
    fn rewrite_args_does_not_duplicate_format() {
        let handler = RubocopHandler;
        let out = handler.rewrite_args(&args(&["rubocop", "--format", "progress"]));
        let count = out.iter().filter(|a| a.as_str() == "--format").count();
        assert_eq!(count, 1);
    }

    #[test]
    fn parse_json_no_offenses() {
        let json = serde_json::json!({
            "files": [],
            "summary": { "inspected_file_count": 5, "offense_count": 0 }
        });
        let result = parse_rubocop_json(&json).unwrap();
        assert!(result.contains("no offenses"), "got: {}", result);
    }

    #[test]
    fn parse_json_groups_by_severity() {
        let json = serde_json::json!({
            "files": [{
                "path": "app/foo.rb",
                "offenses": [
                    {
                        "severity": "error",
                        "message": "Syntax error",
                        "cop_name": "Lint/Syntax",
                        "location": { "line": 10 }
                    },
                    {
                        "severity": "convention",
                        "message": "Line is too long",
                        "cop_name": "Layout/LineLength",
                        "location": { "line": 20 }
                    }
                ]
            }],
            "summary": { "inspected_file_count": 1, "offense_count": 2 }
        });
        let result = parse_rubocop_json(&json).unwrap();
        // Error should appear before convention
        let err_pos  = result.find("Syntax error").unwrap_or(usize::MAX);
        let conv_pos = result.find("Line is too long").unwrap_or(usize::MAX);
        assert!(err_pos < conv_pos, "errors should precede conventions");
        assert!(result.contains("2 offense(s)"), "got: {}", result);
    }
}
