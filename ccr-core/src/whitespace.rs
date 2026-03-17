use crate::config::GlobalConfig;

pub fn normalize(input: &str, config: &GlobalConfig) -> String {
    let lines: Vec<&str> = input.lines().collect();

    // Trim trailing spaces per line
    let lines: Vec<String> = lines.iter().map(|l| l.trim_end().to_string()).collect();

    // Deduplicate consecutive identical lines
    let lines: Vec<String> = if config.deduplicate_lines {
        let mut deduped: Vec<String> = Vec::new();
        for line in lines {
            if deduped.last().map(|l: &String| l.as_str()) != Some(&line) {
                deduped.push(line);
            }
        }
        deduped
    } else {
        lines
    };

    // Collapse multiple consecutive blank lines into one
    let mut result: Vec<String> = Vec::new();
    let mut prev_blank = false;
    for line in &lines {
        let is_blank = line.trim().is_empty();
        if is_blank && prev_blank {
            continue;
        }
        prev_blank = is_blank;
        result.push(line.clone());
    }

    result.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::GlobalConfig;

    fn cfg() -> GlobalConfig {
        GlobalConfig::default()
    }

    #[test]
    fn collapse_multiple_blank_lines() {
        let input = "a\n\n\n\nb";
        assert_eq!(normalize(input, &cfg()), "a\n\nb");
    }

    #[test]
    fn trim_trailing_spaces_per_line() {
        let input = "hello   \nworld  ";
        assert_eq!(normalize(input, &cfg()), "hello\nworld");
    }

    #[test]
    fn deduplicate_consecutive_lines() {
        let input = "foo\nfoo\nfoo\nbar";
        assert_eq!(normalize(input, &cfg()), "foo\nbar");
    }

    #[test]
    fn preserve_intentional_indent() {
        let input = "  indented\n    more";
        let result = normalize(input, &cfg());
        assert!(result.contains("  indented"));
        assert!(result.contains("    more"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(normalize("", &cfg()), "");
    }
}
