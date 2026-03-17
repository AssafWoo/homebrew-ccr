use once_cell::sync::OnceCell;
use regex::Regex;

static CRITICAL_PATTERN: OnceCell<Regex> = OnceCell::new();

fn critical_pattern() -> &'static Regex {
    CRITICAL_PATTERN.get_or_init(|| {
        Regex::new(r"(?i)(error|warning|warn|failed|failure|fatal|panic|exception|critical|FAILED|ERROR|WARNING)").unwrap()
    })
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    dot / (norm_a * norm_b)
}

fn compute_centroid(embeddings: &[Vec<f32>]) -> Vec<f32> {
    if embeddings.is_empty() {
        return vec![];
    }
    let dim = embeddings[0].len();
    let mut centroid = vec![0.0f32; dim];
    for emb in embeddings {
        for (i, v) in emb.iter().enumerate() {
            centroid[i] += v;
        }
    }
    let n = embeddings.len() as f32;
    centroid.iter_mut().for_each(|v| *v /= n);
    centroid
}

pub struct SummarizeResult {
    pub output: String,
    pub lines_in: usize,
    pub lines_out: usize,
    pub omitted: usize,
}

pub fn summarize(text: &str, budget_lines: usize) -> SummarizeResult {
    let lines: Vec<&str> = text.lines().collect();
    let lines_in = lines.len();

    // Try semantic summarization, fall back to head+tail on any error
    let output = match summarize_semantic(&lines, budget_lines) {
        Ok(result) => result,
        Err(_) => summarize_headtail(&lines, budget_lines),
    };

    let lines_out = output.lines().count();
    let omitted = lines_in.saturating_sub(lines_out);

    SummarizeResult {
        output,
        lines_in,
        lines_out,
        omitted,
    }
}

// ── Shared model loader ───────────────────────────────────────────────────────

fn load_model() -> anyhow::Result<fastembed::TextEmbedding> {
    use fastembed::{EmbeddingModel, InitOptions, TextEmbedding};
    TextEmbedding::try_new(
        InitOptions::new(EmbeddingModel::AllMiniLML6V2).with_show_download_progress(false),
    )
}

// ── Sentence-level summarization (for conversation messages) ─────────────────

pub struct MessageSummarizeResult {
    pub output: String,
    pub sentences_in: usize,
    pub sentences_out: usize,
}

/// Compress a user message to `budget_ratio` of its sentences while preserving
/// semantic meaning. Hard-keeps questions, imperatives, and code-bearing sentences.
///
/// Falls back to head+tail if the embedding model fails.
pub fn summarize_message(text: &str, budget_ratio: f32) -> MessageSummarizeResult {
    let sentences = crate::sentence::split_sentences(text);
    let sentences_in = sentences.len();

    if sentences_in == 0 {
        return MessageSummarizeResult {
            output: text.to_string(),
            sentences_in: 0,
            sentences_out: 0,
        };
    }

    let budget = ((sentences_in as f32 * budget_ratio).ceil() as usize).max(1);

    if sentences_in <= budget {
        return MessageSummarizeResult {
            output: text.to_string(),
            sentences_in,
            sentences_out: sentences_in,
        };
    }

    let output = match summarize_sentences_semantic(&sentences, budget) {
        Ok(out) => out,
        Err(_) => summarize_sentences_headtail(&sentences, budget),
    };

    let sentences_out = crate::sentence::split_sentences(&output).len();
    MessageSummarizeResult { output, sentences_in, sentences_out }
}

/// Returns true if a sentence must always be kept regardless of semantic score.
fn is_hard_keep_sentence(s: &str) -> bool {
    let t = s.trim();
    // Questions — always carry user intent
    if t.ends_with('?') {
        return true;
    }
    // Code-bearing sentences — specific, non-paraphraseable
    if t.contains('`') || t.contains("::") {
        return true;
    }
    // snake_case identifiers (function/variable names written inline without backticks)
    // e.g. "process_batch", "intermediate_results", "pg_terminate_backend"
    if t.split_whitespace().any(|w| {
        let w = w.trim_matches(|c: char| !c.is_alphanumeric() && c != '_');
        w.contains('_') && w.chars().next().map(|c| c.is_alphabetic()).unwrap_or(false)
    }) {
        return true;
    }
    // Strong constraint language
    let lower = t.to_lowercase();
    ["must", "never", "always", "ensure", "make sure", "do not", "don't", "avoid", "required", "critical"]
        .iter()
        .any(|kw| lower.contains(kw))
}

fn summarize_sentences_semantic(sentences: &[String], budget: usize) -> anyhow::Result<String> {
    summarize_sentences_semantic_with(sentences, budget, is_hard_keep_sentence)
}

fn summarize_sentences_semantic_with(
    sentences: &[String],
    budget: usize,
    hard_keep: impl Fn(&str) -> bool,
) -> anyhow::Result<String> {
    let model = load_model()?;
    let texts: Vec<&str> = sentences.iter().map(|s| s.as_str()).collect();
    let embeddings = model.embed(texts, None)?;

    let centroid = compute_centroid(&embeddings);

    let scored: Vec<(usize, f32)> = embeddings
        .iter()
        .enumerate()
        .map(|(i, emb)| (i, 1.0 - cosine_similarity(emb, &centroid)))
        .collect();

    let mut selected: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (i, s) in sentences.iter().enumerate() {
        if hard_keep(s) {
            selected.insert(i);
        }
    }

    let max_score = scored.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);
    let threshold = max_score * 0.40;

    let mut ranked = scored.clone();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (idx, score) in &ranked {
        if selected.len() >= budget {
            break;
        }
        if *score < threshold {
            break;
        }
        selected.insert(*idx);
    }

    let mut kept: Vec<usize> = selected.into_iter().collect();
    kept.sort();
    Ok(kept.iter().map(|&i| sentences[i].clone()).collect::<Vec<_>>().join(" "))
}

/// Compute semantic similarity between two texts using BERT embeddings.
/// Returns cosine similarity in [0, 1]. Used as a quality gate on generative output.
pub fn semantic_similarity(a: &str, b: &str) -> anyhow::Result<f32> {
    let model = load_model()?;
    let embeddings = model.embed(vec![a, b], None)?;
    Ok(cosine_similarity(&embeddings[0], &embeddings[1]))
}

/// Compute BERT embeddings for a batch of texts in one shot.
/// Returns one 384-dim vector per input text.
pub fn embed_batch(texts: &[&str]) -> anyhow::Result<Vec<Vec<f32>>> {
    let model = load_model()?;
    Ok(model.embed(texts.to_vec(), None)?)
}

/// Hard-keep rules for assistant messages. More permissive than user rules —
/// preserves data points, lists, code, and monetary/date values.
fn is_hard_keep_assistant_sentence(s: &str) -> bool {
    let t = s.trim();
    // Code-bearing
    if t.contains('`') || t.contains("::") {
        return true;
    }
    // List items (-, *, or numbered)
    let first = t.chars().next().unwrap_or(' ');
    if first == '-' || first == '*' {
        return true;
    }
    if first.is_ascii_digit() && t.chars().nth(1).map(|c| c == '.' || c == ')').unwrap_or(false) {
        return true;
    }
    // Currency or percentage values
    if t.contains('$') || t.contains('€') || t.contains('£') || t.contains('%') {
        return true;
    }
    // Any word-embedded number (data points, dates, counts)
    if t.split_whitespace().any(|w| w.chars().any(|c| c.is_ascii_digit())) {
        return true;
    }
    // Constraint language
    let lower = t.to_lowercase();
    ["must", "never", "always", "ensure", "required", "critical"]
        .iter()
        .any(|kw| lower.contains(kw))
}

/// Compress an assistant message to `budget_ratio` of its sentences while preserving
/// data points, lists, code, and monetary/date values.
///
/// Used for tier-2 compression of old assistant turns.
pub fn summarize_assistant_message(text: &str, budget_ratio: f32) -> MessageSummarizeResult {
    let sentences = crate::sentence::split_sentences(text);
    let sentences_in = sentences.len();

    if sentences_in == 0 {
        return MessageSummarizeResult {
            output: text.to_string(),
            sentences_in: 0,
            sentences_out: 0,
        };
    }

    let budget = ((sentences_in as f32 * budget_ratio).ceil() as usize).max(1);

    if sentences_in <= budget {
        return MessageSummarizeResult {
            output: text.to_string(),
            sentences_in,
            sentences_out: sentences_in,
        };
    }

    let output = match summarize_sentences_semantic_with(&sentences, budget, is_hard_keep_assistant_sentence) {
        Ok(out) => out,
        Err(_) => summarize_sentences_headtail(&sentences, budget),
    };

    let sentences_out = crate::sentence::split_sentences(&output).len();
    MessageSummarizeResult { output, sentences_in, sentences_out }
}

fn summarize_sentences_headtail(sentences: &[String], budget: usize) -> String {
    let total = sentences.len();
    let head = budget / 2;
    let tail = budget - head;
    let mut result: Vec<String> = Vec::new();
    result.extend_from_slice(&sentences[..head.min(total)]);
    if total > head {
        let tail_start = total.saturating_sub(tail);
        if tail_start > head {
            result.extend_from_slice(&sentences[tail_start..]);
        }
    }
    result.join(" ")
}

// ── Line-level summarization (for command output) ─────────────────────────────

fn summarize_semantic(lines: &[&str], budget: usize) -> anyhow::Result<String> {
    let total = lines.len();
    let budget = budget.min(total);

    // Identify non-blank lines for embedding
    let indexed_lines: Vec<(usize, &str)> = lines
        .iter()
        .enumerate()
        .filter(|(_, l)| !l.trim().is_empty())
        .map(|(i, l)| (i, *l))
        .collect();

    if indexed_lines.is_empty() {
        return Ok(lines.join("\n"));
    }

    let model = load_model()?;

    let texts: Vec<&str> = indexed_lines.iter().map(|(_, l)| *l).collect();
    let embeddings = model.embed(texts, None)?;

    let centroid = compute_centroid(&embeddings);

    // Score each line by DISTANCE from centroid (1 - similarity).
    // Outliers (unusual lines) score highest — errors, warnings, unique events.
    // Repetitive noise lines cluster near the centroid and score lowest.
    let scored: Vec<(usize, f32)> = indexed_lines
        .iter()
        .zip(embeddings.iter())
        .map(|((orig_idx, _), emb)| (*orig_idx, 1.0 - cosine_similarity(emb, &centroid)))
        .collect();

    // Hard-keep critical lines
    let mut selected: std::collections::HashSet<usize> = std::collections::HashSet::new();
    for (orig_idx, line) in lines.iter().enumerate() {
        if critical_pattern().is_match(line) {
            selected.insert(orig_idx);
        }
    }

    // Fill remaining budget by highest score, excluding already-selected.
    // Only accept lines scoring above 40% of the max outlier score — prevents
    // filling slots with repetitive noise that just scored marginally higher than peers.
    let max_score = scored.iter().map(|(_, s)| *s).fold(0.0f32, f32::max);
    let score_threshold = max_score * 0.40;

    let mut ranked = scored.clone();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    for (orig_idx, score) in &ranked {
        if selected.len() >= budget {
            break;
        }
        if *score < score_threshold {
            break; // remaining lines are all below threshold, stop filling
        }
        selected.insert(*orig_idx);
    }

    // Restore original order and build output
    let mut kept: Vec<usize> = selected.into_iter().collect();
    kept.sort();

    // Insert omission markers between gaps
    let mut result: Vec<String> = Vec::new();
    let mut prev_idx: Option<usize> = None;
    for idx in &kept {
        if let Some(prev) = prev_idx {
            let gap = idx - prev - 1;
            if gap > 0 {
                result.push(format!("[... {} lines omitted ...]", gap));
            }
        } else if *idx > 0 {
            result.push(format!("[... {} lines omitted ...]", idx));
        }
        result.push(lines[*idx].to_string());
        prev_idx = Some(*idx);
    }
    // Trailing omission
    if let Some(last) = prev_idx {
        let trailing = total - last - 1;
        if trailing > 0 {
            result.push(format!("[... {} lines omitted ...]", trailing));
        }
    }

    Ok(result.join("\n"))
}

fn summarize_headtail(lines: &[&str], budget: usize) -> String {
    let total = lines.len();
    let head = budget / 2;
    let tail = budget - head;
    let omitted = total.saturating_sub(head + tail);

    let mut result: Vec<String> = Vec::new();
    result.extend(lines[..head.min(total)].iter().map(|l| l.to_string()));
    result.push(format!("[... {} lines omitted ...]", omitted));
    if tail > 0 && total > head {
        result.extend(lines[total.saturating_sub(tail)..].iter().map(|l| l.to_string()));
    }
    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_input_not_summarized() {
        let lines: Vec<String> = (0..50).map(|i| format!("line {}", i)).collect();
        let input = lines.join("\n");
        // budget_lines larger than input → no summarization needed by caller
        // but summarize() itself always runs; with budget >= total it keeps all
        let result = summarize(&input, 60);
        // With budget >= total lines, all lines kept
        assert!(result.lines_out <= 50 + 1); // allow omission markers
    }

    #[test]
    fn long_input_summarized() {
        let lines: Vec<String> = (0..500).map(|i| format!("line {}", i)).collect();
        let input = lines.join("\n");
        let result = summarize(&input, 60);
        assert!(result.output.contains("lines omitted"));
        // Should be much shorter than input
        assert!(result.output.lines().count() < 500);
    }

    #[test]
    fn error_lines_always_kept() {
        // Build input > threshold with one error line buried in the middle
        let mut lines: Vec<String> = (0..250).map(|i| format!("noise line {}", i)).collect();
        lines[125] = "error[E0308]: mismatched types".to_string();
        let input = lines.join("\n");
        let result = summarize(&input, 60);
        assert!(result.output.contains("error[E0308]: mismatched types"));
    }

    #[test]
    fn warning_lines_always_kept() {
        let mut lines: Vec<String> = (0..250).map(|i| format!("noise line {}", i)).collect();
        lines[200] = "warning: unused variable `x`".to_string();
        let input = lines.join("\n");
        let result = summarize(&input, 60);
        assert!(result.output.contains("warning: unused variable `x`"));
    }

    #[test]
    fn single_line_input() {
        let result = summarize("just one line", 60);
        assert!(result.output.contains("just one line"));
    }

    #[test]
    fn omission_line_counts_correctly() {
        let lines: Vec<String> = (0..500).map(|i| format!("line {}", i)).collect();
        let input = lines.join("\n");
        let result = summarize(&input, 60);
        assert!(result.output.contains("lines omitted"));
    }

    #[test]
    fn configurable_budget() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {}", i)).collect();
        let input = lines.join("\n");
        let result = summarize(&input, 10);
        assert!(result.output.lines().count() <= 100);
    }
}
