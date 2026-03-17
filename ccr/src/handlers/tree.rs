use super::Handler;

pub struct TreeHandler;

impl Handler for TreeHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        if lines.len() <= 30 {
            return output.to_string();
        }

        // Always keep the last summary line ("N directories, M files")
        let summary = lines
            .iter()
            .rev()
            .find(|l| l.contains("director") && l.contains("file"))
            .map(|l| l.to_string());

        let mut out: Vec<String> = lines.iter().take(25).map(|l| l.to_string()).collect();
        let remaining = lines.len() - 25;
        // Don't count summary line in remaining if present
        let extra = if summary.is_some() {
            remaining.saturating_sub(1)
        } else {
            remaining
        };
        out.push(format!("[... {} more entries]", extra));
        if let Some(s) = summary {
            out.push(s);
        }
        out.join("\n")
    }
}
