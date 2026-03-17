use super::Handler;
use std::collections::BTreeMap;

pub struct GrepHandler;

impl Handler for GrepHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        // Detect if output uses "filename:lineno:match" format (grep -n or rg default)
        let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();

        if lines.is_empty() {
            return output.to_string();
        }

        // Try to group by filename
        let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
        let mut ungrouped: Vec<String> = Vec::new();
        let mut total_matches = 0;

        for line in &lines {
            if let Some((file, rest)) = split_grep_line(line) {
                let entry = by_file.entry(file).or_default();
                let truncated = truncate_line(rest, 120);
                entry.push(truncated);
                total_matches += 1;
            } else {
                // Could be a filename header (rg --heading) or match without file
                ungrouped.push(truncate_line(line, 120));
                total_matches += 1;
            }
        }

        if by_file.is_empty() {
            // No file grouping possible
            let shown = 50.min(ungrouped.len());
            let extra = ungrouped.len().saturating_sub(50);
            let mut out: Vec<String> = ungrouped[..shown].to_vec();
            if extra > 0 {
                out.push(format!("[+{} more matches]", extra));
            }
            return out.join("\n");
        }

        let file_count = by_file.len();
        let mut out: Vec<String> = Vec::new();
        let mut shown = 0;
        const LIMIT: usize = 50;

        'outer: for (file, matches) in &by_file {
            out.push(format!("{}:", file));
            for m in matches {
                if shown >= LIMIT {
                    break 'outer;
                }
                out.push(format!("  {}", m));
                shown += 1;
            }
        }

        if total_matches > LIMIT {
            out.push(format!(
                "[+{} more in {} files]",
                total_matches - shown,
                file_count
            ));
        }

        out.join("\n")
    }
}

/// Attempt to split "file:linenum:content" or "file:content"
fn split_grep_line(line: &str) -> Option<(String, &str)> {
    // Try "filename:N:content" (grep -n) or "filename:content"
    let mut colon_positions = line.match_indices(':');
    if let Some((pos1, _)) = colon_positions.next() {
        let candidate_file = &line[..pos1];
        // If it looks like a path (contains / or . or no spaces)
        if !candidate_file.contains(' ') && !candidate_file.is_empty() {
            let rest = &line[pos1 + 1..];
            // Skip line number if present
            if let Some((pos2, _)) = rest.match_indices(':').next() {
                let maybe_num = &rest[..pos2];
                if maybe_num.chars().all(|c| c.is_ascii_digit()) {
                    return Some((candidate_file.to_string(), &rest[pos2 + 1..]));
                }
            }
            return Some((candidate_file.to_string(), rest));
        }
    }
    None
}

fn truncate_line(line: &str, max: usize) -> String {
    let chars: Vec<char> = line.chars().collect();
    if chars.len() <= max {
        line.to_string()
    } else {
        format!("{}…", chars[..max - 1].iter().collect::<String>())
    }
}
