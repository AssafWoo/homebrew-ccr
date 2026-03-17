use super::util;
use super::Handler;

pub struct DockerHandler;

impl Handler for DockerHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        // For `docker logs`, add --tail 200 if not already specified
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        if subcmd == "logs" && !args.iter().any(|a| a == "--tail") {
            let mut out = args.to_vec();
            // Insert --tail 200 after "logs"
            out.insert(2, "200".to_string());
            out.insert(2, "--tail".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        // Handle compose subcommands
        let effective_subcmd = if subcmd == "compose" || subcmd == "stack" {
            args.get(2).map(|s| s.as_str()).unwrap_or("")
        } else {
            subcmd
        };

        match effective_subcmd {
            "logs" => filter_logs(output),
            "ps" => filter_ps(output),
            "images" => filter_images(output),
            _ => output.to_string(),
        }
    }
}

/// Semantic deduplication using BERT embeddings.
/// Falls back to exact-match dedup if BERT unavailable.
fn semantic_dedup(lines: &[&str]) -> Vec<String> {
    // Hard-keep lines: errors, stack traces, first 5 and last 5
    let is_hard_keep = |line: &str| -> bool {
        let l = line.to_lowercase();
        l.contains("error")
            || l.contains("panic")
            || l.contains("fatal")
            || l.contains("exception")
            || l.contains("failed")
            || l.contains("stack trace")
            || l.contains("caused by")
            || l.contains("at ")
    };

    // Try BERT dedup
    let non_empty: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, *l))
        .collect();

    if non_empty.is_empty() {
        return lines.iter().map(|l| l.to_string()).collect();
    }

    let texts: Vec<&str> = non_empty.iter().map(|(_, l)| *l).collect();

    match ccr_core::summarizer::embed_batch(&texts) {
        Ok(embeddings) => {
            let threshold = 0.90f32;
            let mut kept_indices: Vec<usize> = Vec::new();
            let mut kept_embeddings: Vec<Vec<f32>> = Vec::new();

            for (pos, (orig_idx, line)) in non_empty.iter().enumerate() {
                // Always keep hard-keep lines
                if is_hard_keep(line) {
                    kept_indices.push(*orig_idx);
                    kept_embeddings.push(embeddings[pos].clone());
                    continue;
                }

                // Check similarity against already-kept embeddings
                let is_dup = kept_embeddings.iter().any(|kept_emb| {
                    util::cosine_similarity(&embeddings[pos], kept_emb) > threshold
                });

                if !is_dup {
                    kept_indices.push(*orig_idx);
                    kept_embeddings.push(embeddings[pos].clone());
                }
            }

            kept_indices.sort();
            kept_indices.iter().map(|&i| lines[i].to_string()).collect()
        }
        Err(_) => {
            // Fall back to exact-match dedup
            let mut seen = std::collections::HashSet::new();
            lines
                .iter()
                .filter(|&&l| seen.insert(l))
                .map(|l| l.to_string())
                .collect()
        }
    }
}


fn filter_logs(output: &str) -> String {
    let lines: Vec<&str> = output.lines().collect();
    let deduped = semantic_dedup(&lines);

    let original_count = lines.len();
    let deduped_count = deduped.len();

    let mut result = deduped.join("\n");
    if deduped_count < original_count {
        result.push_str(&format!(
            "\n[{} duplicate lines collapsed by semantic dedup]",
            original_count - deduped_count
        ));
    }
    result
}

fn filter_ps(output: &str) -> String {
    // Keep only name/container ID, status, and ports columns
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return output.to_string();
    }

    // Header + data rows
    let mut out: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            // Header: keep it but truncate
            out.push(line.to_string());
        } else if !line.trim().is_empty() {
            // Data row: extract name, status, ports
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 5 {
                // Typical docker ps columns: CONTAINER ID, IMAGE, COMMAND, CREATED, STATUS, PORTS, NAMES
                let name = parts.last().unwrap_or(&"");
                let status = parts[4];
                // Try to get ports (may span multiple columns)
                let ports_start = line.rfind("  ").unwrap_or(0);
                let ports = &line[ports_start..].trim();
                out.push(format!("{} [{}] {}", name, status, ports));
            } else {
                out.push(line.to_string());
            }
        }
    }
    out.join("\n")
}

fn filter_images(output: &str) -> String {
    // Keep only repo, tag, and size
    let lines: Vec<&str> = output.lines().collect();
    if lines.is_empty() {
        return output.to_string();
    }

    let mut out: Vec<String> = Vec::new();
    for (i, line) in lines.iter().enumerate() {
        if i == 0 {
            out.push("REPOSITORY           TAG       SIZE".to_string());
        } else if !line.trim().is_empty() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 7 {
                // REPOSITORY TAG IMAGE_ID CREATED VIRTUAL_SIZE
                let repo = parts[0];
                let tag = parts[1];
                let size = parts.last().unwrap_or(&"");
                out.push(format!("{:<20} {:<9} {}", repo, tag, size));
            } else {
                out.push(line.to_string());
            }
        }
    }
    out.join("\n")
}
