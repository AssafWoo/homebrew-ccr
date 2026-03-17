use anyhow::Result;
use std::collections::BTreeMap;
use std::path::Path;

struct Opportunity {
    command: String,
    total_output_bytes: usize,
    call_count: usize,
    savings_pct: f32,
}

pub fn run() -> Result<()> {
    let projects_dir = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?
        .join(".claude")
        .join("projects");

    if !projects_dir.exists() {
        println!("No Claude Code history found at {}", projects_dir.display());
        return Ok(());
    }

    // Collect all JSONL files
    let mut jsonl_files: Vec<std::path::PathBuf> = Vec::new();
    collect_jsonl(&projects_dir, &mut jsonl_files);

    if jsonl_files.is_empty() {
        println!("No session history found in {}", projects_dir.display());
        return Ok(());
    }

    // Aggregate by command: track total output size for unoptimized calls
    let mut by_cmd: BTreeMap<String, (usize, usize)> = BTreeMap::new(); // cmd -> (total_bytes, count)

    for path in &jsonl_files {
        scan_jsonl(path, &mut by_cmd);
    }

    if by_cmd.is_empty() {
        println!("No unoptimized Bash commands found in history.");
        return Ok(());
    }

    // Compute estimated savings per command
    let handler_savings: &[(&str, f32)] = &[
        ("cargo", 0.87),
        ("curl", 0.96),
        ("git", 0.80),
        ("docker", 0.85),
        ("npm", 0.85),
        ("pnpm", 0.85),
        ("yarn", 0.85),
        ("ls", 0.80),
        ("cat", 0.70),
        ("grep", 0.80),
        ("rg", 0.80),
        ("find", 0.78),
    ];

    let savings_map: BTreeMap<&str, f32> = handler_savings.iter().cloned().collect();

    let mut opportunities: Vec<Opportunity> = by_cmd
        .iter()
        .filter_map(|(cmd, (bytes, count))| {
            let savings_pct = *savings_map.get(cmd.as_str()).unwrap_or(&0.40); // BERT fallback
            if savings_pct > 0.0 && *bytes > 0 {
                Some(Opportunity {
                    command: cmd.clone(),
                    total_output_bytes: *bytes,
                    call_count: *count,
                    savings_pct: savings_pct * 100.0,
                })
            } else {
                None
            }
        })
        .collect();

    // Sort by estimated savings descending
    opportunities.sort_by(|a, b| {
        let a_saved = (a.total_output_bytes as f32 * a.savings_pct / 100.0) as usize;
        let b_saved = (b.total_output_bytes as f32 * b.savings_pct / 100.0) as usize;
        b_saved.cmp(&a_saved)
    });

    if opportunities.is_empty() {
        println!("All detected commands are already optimized with ccr run.");
        return Ok(());
    }

    println!("CCR Discover — Missed Savings");
    println!("==============================");
    println!(
        "{:<12} {:>6} {:>12} {:>8}  {}",
        "COMMAND", "CALLS", "OUTPUT", "SAVINGS", "IMPACT"
    );
    println!("{}", "-".repeat(55));

    let mut total_potential_bytes: usize = 0;
    for opp in &opportunities {
        let saved_bytes =
            (opp.total_output_bytes as f32 * opp.savings_pct / 100.0) as usize;
        total_potential_bytes += saved_bytes;

        let bar_len = (opp.savings_pct / 5.0) as usize; // 20 chars = 100%
        let bar = "█".repeat(bar_len.min(20));

        println!(
            "{:<12} {:>6} {:>12} {:>7.0}%  {}",
            opp.command,
            opp.call_count,
            human_bytes(opp.total_output_bytes),
            opp.savings_pct,
            bar,
        );
    }

    println!("{}", "-".repeat(55));
    println!(
        "Potential savings: {} bytes across {} command types",
        human_bytes(total_potential_bytes),
        opportunities.len()
    );
    println!();
    println!("Run `ccr init` to enable PreToolUse auto-rewriting.");

    Ok(())
}

fn collect_jsonl(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
    if let Ok(entries) = std::fs::read_dir(dir) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() {
                collect_jsonl(&path, out);
            } else if path.extension().map(|e| e == "jsonl").unwrap_or(false) {
                out.push(path);
            }
        }
    }
}

fn scan_jsonl(path: &Path, by_cmd: &mut BTreeMap<String, (usize, usize)>) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    for line in content.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(line) else {
            continue;
        };

        // Look for tool calls: { tool_input: { command: "..." }, tool_response: { output: "..." } }
        let cmd_str = v
            .get("tool_input")
            .and_then(|ti| ti.get("command"))
            .and_then(|c| c.as_str());

        let output_str = v
            .get("tool_response")
            .and_then(|tr| tr.get("output"))
            .and_then(|o| o.as_str());

        let Some(cmd) = cmd_str else { continue };

        // Skip already-optimized commands
        let trimmed = cmd.trim();
        if trimmed.starts_with("ccr ") {
            continue;
        }

        let first = trimmed.split_whitespace().next().unwrap_or("");
        if first.is_empty() {
            continue;
        }

        let output_bytes = output_str.map(|o| o.len()).unwrap_or(0);
        let entry = by_cmd.entry(first.to_string()).or_insert((0, 0));
        entry.0 += output_bytes;
        entry.1 += 1;
    }
}

fn human_bytes(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{}B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1}KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1}MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
