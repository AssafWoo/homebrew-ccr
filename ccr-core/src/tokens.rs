use once_cell::sync::Lazy;
use tiktoken_rs::cl100k_base;
use tiktoken_rs::CoreBPE;

static ENCODER: Lazy<CoreBPE> = Lazy::new(|| cl100k_base().unwrap());

pub fn count_tokens(text: &str) -> usize {
    ENCODER.encode_with_special_tokens(text).len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_string_is_zero_tokens() {
        assert_eq!(count_tokens(""), 0);
    }

    #[test]
    fn known_string_token_count() {
        // "hello world" is typically 2 tokens in cl100k_base
        let count = count_tokens("hello world");
        assert!(count > 0);
        assert!(count <= 5);
    }

    #[test]
    fn count_increases_with_longer_input() {
        let short = count_tokens("hello");
        let long = count_tokens("hello world this is a longer sentence with many more words");
        assert!(long > short);
    }

    #[test]
    fn unicode_text_counted() {
        let count = count_tokens("こんにちは世界");
        assert!(count > 0);
    }
}
