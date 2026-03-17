/// Split text into individual sentences.
///
/// Splits on `.`, `!`, `?` followed by whitespace, and on newlines.
/// Simple char-scan approach — no regex lookbehind needed, handles
/// code-like content (backticks, `::`, file paths) without false splits.
pub fn split_sentences(text: &str) -> Vec<String> {
    let mut sentences: Vec<String> = Vec::new();
    let mut current = String::new();
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        let ch = chars[i];
        current.push(ch);

        let is_terminal = matches!(ch, '.' | '!' | '?');
        let next_is_space = i + 1 < len && chars[i + 1].is_whitespace();
        let at_end = i + 1 == len;

        if ch == '\n' || (is_terminal && (next_is_space || at_end)) {
            let trimmed = current.trim().to_string();
            if !trimmed.is_empty() {
                sentences.push(trimmed);
            }
            current = String::new();
            // Skip following horizontal whitespace (not newlines — they split too)
            if next_is_space && ch != '\n' {
                i += 1;
                while i + 1 < len && chars[i + 1] == ' ' {
                    i += 1;
                }
            }
        }

        i += 1;
    }

    let trimmed = current.trim().to_string();
    if !trimmed.is_empty() {
        sentences.push(trimmed);
    }

    sentences
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn splits_basic_sentences() {
        let s = split_sentences("Hello world. This is a test. It works!");
        assert_eq!(s.len(), 3);
        assert_eq!(s[0], "Hello world.");
        assert_eq!(s[2], "It works!");
    }

    #[test]
    fn splits_on_newline() {
        let s = split_sentences("First line\nSecond line\nThird line");
        assert_eq!(s.len(), 3);
    }

    #[test]
    fn splits_on_question_mark() {
        let s = split_sentences("What is this? It is a test.");
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn handles_empty() {
        assert!(split_sentences("").is_empty());
    }

    #[test]
    fn handles_single_sentence_no_terminal() {
        let s = split_sentences("just a fragment");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], "just a fragment");
    }

    #[test]
    fn preserves_code_tokens() {
        let s = split_sentences("Use `budget_ratio = 0.20` for tier 2. This is important.");
        assert!(s[0].contains("budget_ratio"));
        assert_eq!(s.len(), 2);
    }

    #[test]
    fn skips_blank_segments() {
        let s = split_sentences("  \n  \nHello.\n  ");
        assert_eq!(s.len(), 1);
        assert_eq!(s[0], "Hello.");
    }
}
