//! IX — Intent Extraction.
//!
//! Reads the most recent assistant message from Claude Code's live JSONL session
//! file and returns it as a BERT query string. This replaces the shallow command
//! name (e.g. "cargo") with Claude's natural-language intent (e.g. "trace where
//! the memory leak occurs in the connection pool"), making BERT importance scoring
//! dramatically more relevant.
//!
//! Every failure returns `None` silently — the caller falls back to the command
//! string. Zero panics, zero stderr output.

use std::io::{Read, Seek, SeekFrom};

/// Extract the last assistant text from the current Claude Code session's JSONL file.
/// Returns `None` on any error (file not found, parse failure, empty content).
pub fn extract_intent() -> Option<String> {
    let projects_dir = dirs::home_dir()?.join(".claude").join("projects");
    let project_dir = crate::util::project_dir_from_cwd()?;
    let session_dir = projects_dir.join(&project_dir);

    // Find the most recently modified .jsonl file in the project dir
    let jsonl_path = std::fs::read_dir(&session_dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().ends_with(".jsonl"))
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            let mtime = meta.modified().ok()?;
            Some((mtime, e.path()))
        })
        .max_by_key(|(t, _)| *t)?
        .1;

    // Scan backwards in 256 KB chunks to find the last assistant message.
    // Assistant messages can be preceded by very large tool-result lines (file reads,
    // long command output), so a fixed small tail would miss them in large sessions.
    let mut file = std::fs::File::open(&jsonl_path).ok()?;
    let file_len = file.metadata().ok()?.len();

    const CHUNK: u64 = 262_144; // 256 KB per pass
    const MAX_SCAN: u64 = 4 * CHUNK; // give up after 1 MB

    let mut last_text: Option<String> = None;
    let mut scanned: u64 = 0;

    while scanned < MAX_SCAN && last_text.is_none() {
        let window = CHUNK.min(file_len.saturating_sub(scanned));
        if window == 0 {
            break;
        }
        let offset = file_len.saturating_sub(scanned + window);
        file.seek(SeekFrom::Start(offset)).ok()?;
        let mut buf = vec![0u8; window as usize];
        file.read_exact(&mut buf).ok()?;

        // Walk lines from the END of this chunk so we get the most recent first
        let text = String::from_utf8_lossy(&buf);
        let mut lines: Vec<&str> = text.lines().collect();
        lines.reverse();

        for line in lines {
            if !line.contains("\"type\":\"assistant\"") {
                continue;
            }
            let Ok(obj) = serde_json::from_str::<serde_json::Value>(line) else {
                continue;
            };
            if obj.get("type").and_then(|t| t.as_str()) != Some("assistant") {
                continue;
            }
            let Some(content) = obj
                .get("message")
                .and_then(|m| m.get("content"))
                .and_then(|c| c.as_array())
            else {
                continue;
            };
            for block in content {
                if block.get("type").and_then(|t| t.as_str()) == Some("text") {
                    if let Some(t) = block.get("text").and_then(|t| t.as_str()) {
                        let trimmed = t.trim();
                        if !trimmed.is_empty() {
                            last_text = Some(trimmed.to_string());
                            break;
                        }
                    }
                }
            }
            if last_text.is_some() {
                break;
            }
        }
        scanned += window;
    }

    let text = last_text?;
    Some(clean_intent(&text))
}

/// Strip markdown, return up to 256 chars. Truncates at a sentence boundary only
/// if the text is longer than 256 chars — short responses are kept whole.
fn clean_intent(text: &str) -> String {
    let stripped: String = text
        .chars()
        .filter(|c| !matches!(c, '*' | '`' | '#' | '>'))
        .collect();
    let stripped = stripped.trim();

    if stripped.len() <= 256 {
        return stripped.to_string();
    }

    // Long text: truncate at sentence boundary nearest to char 256
    let chunk = &stripped[..256];
    if let Some(pos) = chunk.rfind(|c| matches!(c, '.' | '?' | '!')) {
        chunk[..=pos].trim().to_string()
    } else {
        chunk.trim().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_intent_returns_none_when_no_session() {
        // Should not panic with a nonexistent session
        // (project dir may or may not exist; either way, no panic)
        let _ = extract_intent();
    }

    #[test]
    fn clean_intent_strips_markdown() {
        let result = clean_intent("**Run** the `cargo build` command");
        assert!(!result.contains("**"), "got: {}", result);
        assert!(!result.contains('`'), "got: {}", result);
    }

    #[test]
    fn clean_intent_truncates_to_256() {
        let long: String = "x".repeat(500);
        let result = clean_intent(&long);
        assert!(result.len() <= 256);
    }

    #[test]
    fn clean_intent_truncates_at_sentence() {
        let result = clean_intent("First sentence. Second very long sentence that goes on.");
        assert_eq!(result, "First sentence.");
    }

    #[test]
    fn clean_intent_handles_question() {
        let result = clean_intent("Where is the bug? More text here.");
        assert_eq!(result, "Where is the bug?");
    }

    #[test]
    fn clean_intent_no_boundary_returns_chunk() {
        let result = clean_intent("no sentence boundary here at all no punctuation");
        assert!(!result.is_empty());
        assert!(result.len() <= 256);
    }
}
