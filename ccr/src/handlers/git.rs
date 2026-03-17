use super::Handler;

pub struct GitHandler;

impl Handler for GitHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        if subcmd == "log" {
            // Inject --oneline if not already present
            if !args.iter().any(|a| a == "--oneline") {
                let mut out = args.to_vec();
                out.insert(2, "--oneline".to_string());
                return out;
            }
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "status" => filter_status(output),
            "log" => filter_log(output),
            "diff" => filter_diff(output),
            "push" | "pull" | "fetch" => filter_push_pull(output),
            "commit" | "add" => filter_commit(output),
            "branch" | "stash" => filter_list(output),
            _ => output.to_string(),
        }
    }
}

fn filter_status(output: &str) -> String {
    let lines: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            // Keep lines that describe changed or untracked files, and branch info
            !t.is_empty()
                && !t.starts_with("no changes added")
                && !t.starts_with("nothing to commit")
                && !t.starts_with("(use \"git")
                && !t.starts_with("nothing added")
        })
        .collect();

    if lines.len() > 20 {
        let shown = &lines[..20];
        let extra = lines.len() - 20;
        let mut out: Vec<String> = shown.iter().map(|l| l.to_string()).collect();
        out.push(format!("[+{} more files]", extra));
        return out.join("\n");
    }

    if lines.is_empty() {
        return "nothing to commit, working tree clean".to_string();
    }

    lines.join("\n")
}

fn filter_log(output: &str) -> String {
    // With --oneline, each line is "hash message"
    let lines: Vec<&str> = output.lines().take(20).collect();
    let total = output.lines().count();
    let mut out = lines.join("\n");
    if total > 20 {
        out.push_str(&format!("\n[+{} more commits]", total - 20));
    }
    out
}

fn filter_diff(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        if line.starts_with("+++")
            || line.starts_with("---")
            || line.starts_with("diff ")
            || line.starts_with("index ")
            || line.starts_with("@@")
            || line.starts_with('+')
            || line.starts_with('-')
        {
            out.push(line.to_string());
        }
        // Skip context lines (lines starting with space or empty)
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_push_pull(output: &str) -> String {
    // Keep branch info and summary lines
    let important: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && !t.starts_with("remote: Counting")
                && !t.starts_with("remote: Compressing")
                && !t.starts_with("remote: Enumerating")
                && !t.starts_with("Counting objects")
                && !t.starts_with("Compressing objects")
                && !t.starts_with("Writing objects")
                && !t.starts_with("Delta compression")
        })
        .collect();

    if important.is_empty() {
        output.to_string()
    } else {
        important.join("\n")
    }
}

fn filter_commit(output: &str) -> String {
    // Extract the one-liner summary: "[branch hash] message"
    let summary: Vec<&str> = output
        .lines()
        .filter(|l| l.trim().starts_with('[') || l.contains("file") || l.contains("insertion"))
        .collect();
    if summary.is_empty() {
        output.to_string()
    } else {
        summary.join("\n")
    }
}

fn filter_list(output: &str) -> String {
    let lines: Vec<&str> = output.lines().filter(|l| !l.trim().is_empty()).collect();
    if lines.len() > 30 {
        let shown = &lines[..30];
        let extra = lines.len() - 30;
        let mut out: Vec<String> = shown.iter().map(|l| l.to_string()).collect();
        out.push(format!("[+{} more]", extra));
        out.join("\n")
    } else {
        lines.join("\n")
    }
}
