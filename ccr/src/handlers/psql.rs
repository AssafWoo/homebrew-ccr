use super::Handler;

pub struct PsqlHandler;

impl Handler for PsqlHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.is_empty() {
            return output.to_string();
        }

        // Keep psql ERROR lines always
        let has_error = lines.iter().any(|l| l.trim().starts_with("ERROR:") || l.trim().starts_with("FATAL:"));
        if has_error {
            let errors: Vec<&str> = lines
                .iter()
                .filter(|l| {
                    let t = l.trim();
                    t.starts_with("ERROR:") || t.starts_with("FATAL:") || t.starts_with("DETAIL:") || t.starts_with("HINT:")
                })
                .copied()
                .collect();
            return errors.join("\n");
        }

        // Strip +----+ border lines and process as table
        let data_lines: Vec<&str> = lines
            .iter()
            .filter(|l| {
                let t = l.trim();
                !t.is_empty() && !t.chars().all(|c| c == '+' || c == '-' || c == '=')
            })
            .copied()
            .collect();

        if data_lines.is_empty() {
            return output.to_string();
        }

        // Strip leading/trailing | from each line
        let cleaned: Vec<String> = data_lines
            .iter()
            .map(|l| {
                let t = l.trim();
                let s = if t.starts_with('|') { &t[1..] } else { t };
                let s = if s.ends_with('|') { &s[..s.len() - 1] } else { s };
                // Normalize multiple spaces between columns
                s.trim().to_string()
            })
            .collect();

        let total = cleaned.len();
        const MAX_ROWS: usize = 20;

        if total <= MAX_ROWS + 1 {
            // +1 for header
            return cleaned.join("\n");
        }

        // Header + first MAX_ROWS data rows + truncation note
        let mut out: Vec<String> = Vec::new();
        out.push(cleaned[0].clone()); // header
        for row in cleaned.iter().skip(1).take(MAX_ROWS) {
            out.push(row.clone());
        }
        let remaining = total - 1 - MAX_ROWS;
        if remaining > 0 {
            out.push(format!("[+{} more rows]", remaining));
        }
        // Keep last line if it's a row count "(N rows)"
        if let Some(last) = cleaned.last() {
            if last.trim().starts_with('(') && last.contains("row") {
                out.push(last.clone());
            }
        }
        out.join("\n")
    }
}
