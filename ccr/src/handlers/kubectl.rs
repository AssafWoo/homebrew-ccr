use super::Handler;
use super::util;

pub struct KubectlHandler;

impl Handler for KubectlHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        if subcmd == "logs" && !args.iter().any(|a| a.starts_with("--tail")) {
            let mut out = args.to_vec();
            out.push("--tail=200".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "get" => util::compact_table(output, &[0, 1, 4]),
            "logs" => filter_logs(output),
            "describe" => filter_describe(output),
            "apply" | "delete" | "rollout" => filter_changes(output),
            _ => output.to_string(),
        }
    }
}

fn filter_logs(output: &str) -> String {
    let is_hard_keep = |line: &str| -> bool {
        let l = line.to_lowercase();
        l.contains("error")
            || l.contains("panic")
            || l.contains("fatal")
            || l.contains("exception")
            || l.contains("failed")
    };

    let lines: Vec<&str> = output.lines().collect();
    let non_empty: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, *l))
        .collect();

    if non_empty.is_empty() {
        return output.to_string();
    }

    let texts: Vec<&str> = non_empty.iter().map(|(_, l)| *l).collect();

    match ccr_core::summarizer::embed_batch(&texts) {
        Ok(embeddings) => {
            let threshold = 0.90f32;
            let mut kept_indices: Vec<usize> = Vec::new();
            let mut kept_embeddings: Vec<Vec<f32>> = Vec::new();

            for (pos, (orig_idx, line)) in non_empty.iter().enumerate() {
                if is_hard_keep(line) {
                    kept_indices.push(*orig_idx);
                    kept_embeddings.push(embeddings[pos].clone());
                    continue;
                }
                let is_dup = kept_embeddings
                    .iter()
                    .any(|e| util::cosine_similarity(&embeddings[pos], e) > threshold);
                if !is_dup {
                    kept_indices.push(*orig_idx);
                    kept_embeddings.push(embeddings[pos].clone());
                }
            }

            kept_indices.sort();
            let original_count = lines.len();
            let deduped: Vec<String> = kept_indices.iter().map(|&i| lines[i].to_string()).collect();
            let mut result = deduped.join("\n");
            if deduped.len() < original_count {
                result.push_str(&format!(
                    "\n[{} duplicate lines collapsed by semantic dedup]",
                    original_count - deduped.len()
                ));
            }
            result
        }
        Err(_) => {
            let mut seen = std::collections::HashSet::new();
            lines
                .iter()
                .filter(|&&l| seen.insert(l))
                .cloned()
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn filter_describe(output: &str) -> String {
    let keep_sections = ["Name:", "Status:", "Conditions:", "Events:"];
    let mut out: Vec<String> = Vec::new();
    let mut in_section = false;
    let mut annotation_count = 0usize;
    let mut in_annotations = false;

    for line in output.lines() {
        let t = line.trim();

        // Check if we're starting an annotation/label block
        if t.starts_with("Annotations:") || t.starts_with("Labels:") {
            in_annotations = true;
            annotation_count = 0;
            out.push(line.to_string());
            continue;
        }

        if in_annotations {
            // Indented continuation lines are annotation entries
            if line.starts_with(' ') || line.starts_with('\t') {
                annotation_count += 1;
                if annotation_count <= 5 {
                    out.push(line.to_string());
                } else if annotation_count == 6 {
                    out.push(format!("[{} annotations]", annotation_count));
                }
                continue;
            } else {
                in_annotations = false;
            }
        }

        let is_section = keep_sections.iter().any(|s| t.starts_with(s));
        if is_section {
            in_section = true;
        } else if !t.is_empty() && !line.starts_with(' ') && !line.starts_with('\t') {
            in_section = false;
        }

        if in_section || is_section {
            out.push(line.to_string());
        }
    }

    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_changes(output: &str) -> String {
    let out: Vec<&str> = output
        .lines()
        .filter(|l| {
            let t = l.trim();
            !t.is_empty()
                && (t.contains("created")
                    || t.contains("deleted")
                    || t.contains("configured")
                    || t.contains("unchanged")
                    || t.contains("error")
                    || t.contains("Error")
                    || t.starts_with("deployment.")
                    || t.starts_with("service.")
                    || t.starts_with("pod/"))
        })
        .collect();
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}
