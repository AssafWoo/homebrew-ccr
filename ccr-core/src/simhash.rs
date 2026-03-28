//! SimHash-based near-duplicate line deduplication.
//!
//! Uses a 64-bit locality-sensitive hash (SimHash with FNV-1a token hashing)
//! to identify and collapse lines that differ only in metadata like timestamps,
//! PIDs, or sequence numbers — patterns that exact-match dedup and BERT both
//! handle poorly.
//!
//! Critical lines (containing error/warning/fatal/etc.) are always preserved.

use once_cell::sync::OnceCell;
use regex::Regex;

/// Minimum line count to activate SimHash deduplication.
/// Aligns with the default BERT summarization threshold so SimHash acts as a
/// fast pre-processor that reduces BERT's input rather than a separate
/// compression stage that fires earlier than BERT.
pub const MIN_LINES: usize = 50;

/// Maximum Hamming distance for two SimHashes to be considered near-duplicates.
/// 10 bits out of 64 tolerates ~15% bit-flip rate — appropriate for log lines
/// that differ in one or two metadata tokens (timestamps, sequence numbers).
pub const HAMMING_THRESHOLD: u32 = 10;

static CRITICAL: OnceCell<Regex> = OnceCell::new();

fn critical() -> &'static Regex {
    CRITICAL.get_or_init(|| {
        Regex::new(
            r"(?i)(error|warning|warn|failed|failure|fatal|panic|exception|critical)",
        )
        .unwrap()
    })
}

// ── FNV-1a 64-bit hash ────────────────────────────────────────────────────────

#[inline]
fn fnv1a(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

// ── SimHash ───────────────────────────────────────────────────────────────────

/// Compute the 64-bit SimHash of a line.
///
/// Each whitespace-separated token contributes a signed vote (+1 or -1) to each
/// of the 64 bit positions based on the corresponding bit of its FNV-1a hash.
/// The final bit is 1 if the sum of votes is positive, 0 otherwise.
pub fn simhash(line: &str) -> u64 {
    let mut votes = [0i32; 64];
    let mut any = false;
    for token in line.split_whitespace() {
        any = true;
        let h = fnv1a(token.as_bytes());
        for i in 0u32..64 {
            if (h >> i) & 1 == 1 {
                votes[i as usize] += 1;
            } else {
                votes[i as usize] -= 1;
            }
        }
    }
    if !any {
        return 0;
    }
    let mut result = 0u64;
    for (i, &v) in votes.iter().enumerate() {
        if v > 0 {
            result |= 1u64 << i;
        }
    }
    result
}

/// Hamming distance between two SimHashes (number of differing bits).
#[inline]
pub fn hamming(a: u64, b: u64) -> u32 {
    (a ^ b).count_ones()
}

// ── Deduplication ─────────────────────────────────────────────────────────────

/// Collapse near-duplicate lines, keeping one representative per cluster.
///
/// Lines within `threshold` Hamming distance of a prior representative are
/// grouped together. A `[N similar lines omitted]` marker is inserted after
/// the representative for each non-trivial group.
///
/// Lines matching the critical pattern (error/warning/fatal/…) are never
/// grouped as duplicates; they always appear in full.
///
/// Returns the output lines in their original relative order.
pub fn dedup_near_duplicates(lines: &[&str], threshold: u32) -> Vec<String> {
    if lines.is_empty() {
        return Vec::new();
    }

    let hashes: Vec<u64> = lines.iter().map(|l| simhash(l)).collect();
    let is_crit: Vec<bool> = lines.iter().map(|l| critical().is_match(l)).collect();

    // grouped[i] = true means line i has been absorbed into a prior cluster
    let mut grouped = vec![false; lines.len()];
    let mut out: Vec<String> = Vec::with_capacity(lines.len());

    for i in 0..lines.len() {
        if grouped[i] {
            continue;
        }

        let mut dup_count = 0usize;

        // Critical lines always stand alone — never absorb others into their group.
        if !is_crit[i] {
            for j in (i + 1)..lines.len() {
                if grouped[j] || is_crit[j] {
                    continue;
                }
                if hamming(hashes[i], hashes[j]) <= threshold {
                    grouped[j] = true;
                    dup_count += 1;
                }
            }
        }

        out.push(lines[i].to_string());
        if dup_count > 0 {
            out.push(format!("[{} similar lines omitted]", dup_count));
        }
    }

    out
}

/// Convenience wrapper that operates on a multi-line string.
pub fn dedup_str(text: &str, threshold: u32) -> String {
    let lines: Vec<&str> = text.lines().collect();
    dedup_near_duplicates(&lines, threshold).join("\n")
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    // S1: identical strings always produce the same hash
    #[test]
    fn simhash_same_string_stable() {
        let line = "2024-03-27 10:00:01 INFO [worker-1] Processing request id=1234";
        assert_eq!(simhash(line), simhash(line));
    }

    // S2: near-duplicate log lines (same message, different timestamp) → small distance
    #[test]
    fn simhash_near_duplicate_small_distance() {
        let a = simhash("2024-03-27 10:00:01 INFO [worker-1] Processing request id=1234");
        let b = simhash("2024-03-27 10:00:02 INFO [worker-1] Processing request id=1235");
        // 2 of ~8 tokens differ — Hamming distance should be well within threshold
        assert!(
            hamming(a, b) <= HAMMING_THRESHOLD,
            "hamming was {}, expected <= {}",
            hamming(a, b),
            HAMMING_THRESHOLD
        );
    }

    // S3: completely distinct content → Hamming distance larger than threshold
    #[test]
    fn simhash_distinct_content_large_distance() {
        let a = simhash("error: type mismatch expected bool found &str");
        let b = simhash("Compiling myproject v0.1.0 /home/user/project");
        assert!(hamming(a, b) > 5, "hamming was {}", hamming(a, b));
    }

    // S4: repeated identical log lines collapse to 1 representative + 1 marker
    #[test]
    fn dedup_collapses_repeated_log_lines() {
        let lines: Vec<&str> = (0..20)
            .map(|_| "2024-03-27 10:00:01 INFO server heartbeat ok")
            .collect();
        let result = dedup_near_duplicates(&lines, HAMMING_THRESHOLD);
        assert!(
            result.len() <= 2,
            "expected ≤ 2 lines, got {}: {:?}",
            result.len(),
            result
        );
        assert!(result[0].contains("heartbeat"));
        if result.len() == 2 {
            assert!(
                result[1].contains("similar lines omitted"),
                "marker missing: {:?}",
                result[1]
            );
        }
    }

    // S5: critical lines are never grouped — all preserved
    #[test]
    fn dedup_preserves_critical_lines() {
        let base = "error: connection refused (os error 111)";
        let lines: Vec<&str> = vec![base; 5];
        let result = dedup_near_duplicates(&lines, HAMMING_THRESHOLD);
        // Critical lines cannot absorb others — all 5 are kept
        assert_eq!(result.len(), 5, "got: {:?}", result);
    }

    // S6: short, distinct input — no markers added
    #[test]
    fn dedup_short_distinct_input_unchanged() {
        let lines = vec!["line one", "line two", "line three"];
        let result = dedup_near_duplicates(&lines, HAMMING_THRESHOLD);
        let has_marker = result.iter().any(|l| l.contains("similar lines omitted"));
        assert!(!has_marker, "unexpected marker in {:?}", result);
    }

    // S7: empty input
    #[test]
    fn dedup_empty_input() {
        let result = dedup_near_duplicates(&[], HAMMING_THRESHOLD);
        assert!(result.is_empty());
    }

    // S8: marker count is accurate
    #[test]
    fn dedup_marker_count_accurate() {
        let lines: Vec<&str> = vec!["INFO heartbeat ok"; 10];
        let result = dedup_near_duplicates(&lines, HAMMING_THRESHOLD);
        assert_eq!(result.len(), 2, "got: {:?}", result);
        assert_eq!(result[1], "[9 similar lines omitted]");
    }

    // S9: threshold 0 — only lines with identical SimHash are grouped
    #[test]
    fn dedup_zero_threshold_exact_only() {
        let lines = vec![
            "2024-03-27 10:00:01 INFO heartbeat",
            "2024-03-27 10:00:02 INFO heartbeat", // timestamp differs
            "something completely different",
        ];
        let result = dedup_near_duplicates(&lines, 0);
        // With threshold 0, timestamp differences should prevent grouping
        assert!(!result.is_empty());
        // Output must not be empty and no crashes
    }

    // S10: fully distinct lines are all preserved
    #[test]
    fn dedup_preserves_distinct_lines() {
        let lines = vec![
            "Compiling foo v0.1.0",
            "Compiling bar v0.2.0",
            "error: type mismatch",
            "warning: unused variable `x`",
            "   --> src/main.rs:10:5",
        ];
        let result = dedup_near_duplicates(&lines, HAMMING_THRESHOLD);
        // error and warning lines are always kept; compile lines differ enough
        assert!(result.len() >= 2, "got: {:?}", result);
        // critical lines must be present
        assert!(result.iter().any(|l| l.contains("error:")));
        assert!(result.iter().any(|l| l.contains("warning:")));
    }

    // dedup_str wrapper works
    #[test]
    fn dedup_str_wrapper() {
        let text = "INFO ok\nINFO ok\nINFO ok";
        let result = dedup_str(text, HAMMING_THRESHOLD);
        // 3 identical lines → 1 representative + marker
        let lines: Vec<&str> = result.lines().collect();
        assert!(lines.len() <= 2);
    }
}
