use super::Handler;

pub struct LsHandler;

impl Handler for LsHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output
            .lines()
            .filter(|l| !l.trim().is_empty())
            .collect();

        if lines.is_empty() {
            return output.to_string();
        }

        // Detect if output is `ls -l` format (starts with "total" or permissions like "drwx")
        let is_long_format = lines.first().map(|l| l.starts_with("total ")).unwrap_or(false)
            || lines
                .first()
                .map(|l| {
                    l.starts_with("dr")
                        || l.starts_with("-r")
                        || l.starts_with("lr")
                        || l.starts_with("-w")
                        || l.starts_with("d-")
                })
                .unwrap_or(false);

        let entries: Vec<LsEntry> = if is_long_format {
            parse_long_format(&lines)
        } else {
            parse_short_format(&lines)
        };

        // Sort: dirs first, then files
        let mut dirs: Vec<&LsEntry> = entries.iter().filter(|e| e.is_dir).collect();
        let mut files: Vec<&LsEntry> = entries.iter().filter(|e| !e.is_dir).collect();

        dirs.sort_by(|a, b| a.name.cmp(&b.name));
        files.sort_by(|a, b| a.name.cmp(&b.name));

        let total = dirs.len() + files.len();
        let limit = 40;
        let mut shown = 0;
        let mut out: Vec<String> = Vec::new();

        for entry in dirs.iter().chain(files.iter()) {
            if shown >= limit {
                break;
            }
            if entry.is_dir {
                out.push(format!("{}/", entry.name));
            } else {
                out.push(entry.name.clone());
            }
            shown += 1;
        }

        if total > limit {
            out.push(format!("[+{} more]", total - limit));
        }
        out.push(format!(
            "[{} dirs, {} files]",
            dirs.len(),
            files.len()
        ));

        out.join("\n")
    }
}

struct LsEntry {
    name: String,
    is_dir: bool,
}

fn parse_long_format(lines: &[&str]) -> Vec<LsEntry> {
    lines
        .iter()
        .filter(|l| !l.starts_with("total "))
        .filter_map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            if parts.is_empty() {
                return None;
            }
            let is_dir = parts[0].starts_with('d');
            let name = parts.last()?.to_string();
            Some(LsEntry { name, is_dir })
        })
        .collect()
}

fn parse_short_format(lines: &[&str]) -> Vec<LsEntry> {
    lines
        .iter()
        .flat_map(|l| {
            // Space-separated or one-per-line
            l.split_whitespace().map(|name| LsEntry {
                is_dir: name.ends_with('/'),
                name: name.trim_end_matches('/').to_string(),
            })
        })
        .collect()
}
