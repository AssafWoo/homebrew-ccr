use regex::Regex;
use once_cell::sync::Lazy;

static ANSI_ESCAPE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\x1b\[[0-9;]*[A-Za-z]|\x1b[()][0-9A-Za-z]|\x1b[^\x1b]").unwrap()
});

pub fn strip_ansi(input: &str) -> String {
    ANSI_ESCAPE.replace_all(input, "").into_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_basic_color_codes() {
        assert_eq!(strip_ansi("\x1b[32mgreen\x1b[0m"), "green");
    }

    #[test]
    fn strip_cursor_movement() {
        assert_eq!(strip_ansi("\x1b[2J\x1b[Hclear"), "clear");
    }

    #[test]
    fn strip_nested_escape_sequences() {
        assert_eq!(strip_ansi("\x1b[1m\x1b[31mbold red\x1b[0m"), "bold red");
    }

    #[test]
    fn passthrough_plain_text() {
        assert_eq!(strip_ansi("hello world"), "hello world");
    }

    #[test]
    fn empty_string() {
        assert_eq!(strip_ansi(""), "");
    }
}
