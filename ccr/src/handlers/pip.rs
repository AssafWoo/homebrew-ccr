use super::Handler;

pub struct PipHandler;

impl Handler for PipHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");

        match subcmd {
            "freeze" => return output.to_string(),
            "install" | "add" => {
                // Count installed packages, preserve WARNING/ERROR lines
                let mut warnings: Vec<String> = Vec::new();
                let mut installed = 0usize;

                for line in output.lines() {
                    let t = line.trim();
                    if t.starts_with("Successfully installed") {
                        // Count packages in "Successfully installed foo-1.0 bar-2.0 ..."
                        installed += t
                            .trim_start_matches("Successfully installed")
                            .split_whitespace()
                            .count();
                    } else if t.to_uppercase().starts_with("WARNING")
                        || t.to_uppercase().starts_with("ERROR")
                    {
                        warnings.push(line.to_string());
                    }
                }

                let mut out: Vec<String> = warnings;
                if installed > 0 {
                    out.push(format!("[pip install complete — {} packages]", installed));
                } else {
                    // Nothing installed (already satisfied or error)
                    let summary: Vec<&str> = output
                        .lines()
                        .filter(|l| {
                            let t = l.trim();
                            t.contains("already satisfied")
                                || t.contains("Requirement already")
                                || t.to_uppercase().starts_with("ERROR")
                        })
                        .take(5)
                        .collect();
                    if !summary.is_empty() {
                        out.extend(summary.iter().map(|l| l.to_string()));
                    } else {
                        return output.to_string();
                    }
                }
                return out.join("\n");
            }
            _ => {}
        }

        // Default: keep only final status line
        let last = output
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .unwrap_or("");
        if last.is_empty() {
            output.to_string()
        } else {
            last.to_string()
        }
    }
}
