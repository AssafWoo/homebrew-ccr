use super::Handler;

pub struct GhHandler;

impl Handler for GhHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");

        match (subcmd, action) {
            ("pr", "list") => filter_pr_list(output),
            ("pr", "view") => filter_pr_view(output),
            ("pr", "checks") => filter_pr_checks(output),
            ("issue", "list") => filter_issue_list(output),
            ("run", "list") | ("run", "view") => filter_run(output),
            ("repo", "clone") | ("repo", "fork") => last_line(output),
            _ => output.to_string(),
        }
    }
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        format!("{}…", &s[..s.char_indices().nth(max).map(|(i, _)| i).unwrap_or(s.len())])
    }
}

fn filter_pr_list(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let cols: Vec<&str> = t.splitn(5, '\t').collect();
        if cols.len() >= 4 {
            let num = cols[0];
            let title = truncate(cols[1], 60);
            let state = cols[2];
            let author = cols.get(3).unwrap_or(&"");
            out.push(format!("#{} {} [{}] @{}", num, title, state, author));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

fn filter_pr_view(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let mut out: Vec<String> = Vec::new();
    let mut body_lines = 0;
    let mut in_body = false;

    for line in &lines {
        let t = line.trim();
        if t.starts_with("title:") || t.starts_with("state:") || t.starts_with("author:") {
            out.push(line.to_string());
        } else if t.starts_with("--") {
            in_body = true;
        } else if in_body && body_lines < 10 {
            out.push(line.to_string());
            body_lines += 1;
        } else if t.starts_with("checks:") || t.starts_with("review decision:") {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_pr_checks(output: &str) -> String {
    let mut passed = 0usize;
    let mut failed: Vec<String> = Vec::new();

    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with('✓') || t.contains("pass") || t.contains("success") {
            passed += 1;
        } else if t.starts_with('✗') || t.starts_with('×') || t.contains("fail") {
            let name = t.split_whitespace().next().unwrap_or(t);
            failed.push(name.to_string());
        }
    }

    let mut out = format!("✓ {} passed, ✗ {} failed", passed, failed.len());
    if !failed.is_empty() {
        out.push('\n');
        out.push_str(&failed.join("\n"));
    }
    out
}

fn filter_issue_list(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        let cols: Vec<&str> = t.splitn(5, '\t').collect();
        if cols.len() >= 3 {
            let num = cols[0];
            let title = truncate(cols[1], 60);
            let labels = cols.get(2).unwrap_or(&"");
            let assignee = cols.get(3).unwrap_or(&"");
            out.push(format!("#{} {} [{}] @{}", num, title, labels, assignee));
        } else {
            out.push(line.to_string());
        }
    }
    out.join("\n")
}

fn filter_run(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.contains("completed")
            || t.contains("in_progress")
            || t.contains("queued")
            || t.contains("failure")
            || t.contains("success")
            || t.contains("cancelled")
        {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn last_line(output: &str) -> String {
    output
        .lines()
        .rev()
        .find(|l| !l.trim().is_empty())
        .unwrap_or(output)
        .to_string()
}
