use crate::config::{CommandConfig, FilterAction, FilterPattern, SimpleAction};
use regex::Regex;

struct CompiledPattern {
    regex: Regex,
    action: FilterAction,
}

pub struct PatternFilter {
    patterns: Vec<CompiledPattern>,
}

impl PatternFilter {
    pub fn new(config: &CommandConfig) -> anyhow::Result<Self> {
        let mut patterns = Vec::new();
        for p in &config.patterns {
            patterns.push(CompiledPattern {
                regex: Regex::new(&p.regex)?,
                action: p.action.clone(),
            });
        }
        Ok(Self { patterns })
    }

    pub fn apply(&self, input: &str) -> String {
        let lines: Vec<&str> = input.lines().collect();
        let mut result: Vec<String> = Vec::new();

        // For Collapse: track consecutive matches per pattern index
        // We'll do a line-by-line pass tracking collapse state
        let mut collapse_counts: Vec<usize> = vec![0; self.patterns.len()];
        let mut active_collapse: Option<usize> = None; // pattern index currently collapsing

        for line in &lines {
            let mut matched = false;
            for (i, pat) in self.patterns.iter().enumerate() {
                if pat.regex.is_match(line) {
                    matched = true;
                    match &pat.action {
                        FilterAction::Simple(SimpleAction::Remove) => {
                            // flush any active collapse
                            if let Some(ci) = active_collapse {
                                if ci != i {
                                    if collapse_counts[ci] > 0 {
                                        result.push(format!(
                                            "[{} matching lines collapsed]",
                                            collapse_counts[ci]
                                        ));
                                        collapse_counts[ci] = 0;
                                    }
                                    active_collapse = None;
                                }
                            }
                            // just remove — don't add to result
                        }
                        FilterAction::Simple(SimpleAction::Collapse) => {
                            // flush different collapse
                            if let Some(ci) = active_collapse {
                                if ci != i {
                                    if collapse_counts[ci] > 0 {
                                        result.push(format!(
                                            "[{} matching lines collapsed]",
                                            collapse_counts[ci]
                                        ));
                                        collapse_counts[ci] = 0;
                                    }
                                }
                            }
                            active_collapse = Some(i);
                            collapse_counts[i] += 1;
                        }
                        FilterAction::ReplaceWith { ReplaceWith: replacement } => {
                            // flush any active collapse
                            if let Some(ci) = active_collapse {
                                if collapse_counts[ci] > 0 {
                                    result.push(format!(
                                        "[{} matching lines collapsed]",
                                        collapse_counts[ci]
                                    ));
                                    collapse_counts[ci] = 0;
                                }
                                active_collapse = None;
                            }
                            result.push(replacement.clone());
                        }
                    }
                    break;
                }
            }
            if !matched {
                // flush any active collapse
                if let Some(ci) = active_collapse {
                    if collapse_counts[ci] > 0 {
                        result.push(format!(
                            "[{} matching lines collapsed]",
                            collapse_counts[ci]
                        ));
                        collapse_counts[ci] = 0;
                    }
                    active_collapse = None;
                }
                result.push(line.to_string());
            }
        }

        // flush remaining collapse
        if let Some(ci) = active_collapse {
            if collapse_counts[ci] > 0 {
                result.push(format!("[{} matching lines collapsed]", collapse_counts[ci]));
            }
        }

        result.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CommandConfig, FilterAction, FilterPattern, SimpleAction};

    fn make_config(patterns: Vec<FilterPattern>) -> CommandConfig {
        CommandConfig { patterns }
    }

    #[test]
    fn removes_matching_regex_line() {
        let cfg = make_config(vec![FilterPattern {
            regex: "^noise.*".to_string(),
            action: FilterAction::Simple(SimpleAction::Remove),
        }]);
        let filter = PatternFilter::new(&cfg).unwrap();
        let result = filter.apply("noise line\nkeep this");
        assert!(!result.contains("noise line"));
        assert!(result.contains("keep this"));
    }

    #[test]
    fn replaces_matching_line() {
        let cfg = make_config(vec![FilterPattern {
            regex: "^added \\d+ packages.*".to_string(),
            action: FilterAction::ReplaceWith {
                ReplaceWith: "[npm install complete]".to_string(),
            },
        }]);
        let filter = PatternFilter::new(&cfg).unwrap();
        let result = filter.apply("added 42 packages in 5s");
        assert!(result.contains("[npm install complete]"));
    }

    #[test]
    fn collapse_repeated_pattern() {
        let cfg = make_config(vec![FilterPattern {
            regex: "^   Compiling \\S+ v[\\d.]+".to_string(),
            action: FilterAction::Simple(SimpleAction::Collapse),
        }]);
        let filter = PatternFilter::new(&cfg).unwrap();
        let mut lines = Vec::new();
        for i in 0..50 {
            lines.push(format!("   Compiling crate{} v1.0", i));
        }
        let input = lines.join("\n");
        let result = filter.apply(&input);
        // Should have exactly one collapse summary, not 50 lines
        assert!(result.contains("50 matching lines collapsed"));
        let line_count = result.lines().count();
        assert_eq!(line_count, 1);
    }

    #[test]
    fn no_match_passthrough() {
        let cfg = make_config(vec![FilterPattern {
            regex: "^never_matches_xyz$".to_string(),
            action: FilterAction::Simple(SimpleAction::Remove),
        }]);
        let filter = PatternFilter::new(&cfg).unwrap();
        let input = "line one\nline two\nline three";
        assert_eq!(filter.apply(input), input);
    }

    #[test]
    fn multiple_rules_applied_in_order() {
        let cfg = make_config(vec![
            FilterPattern {
                regex: "^noise.*".to_string(),
                action: FilterAction::Simple(SimpleAction::Remove),
            },
            FilterPattern {
                regex: "^replace.*".to_string(),
                action: FilterAction::ReplaceWith {
                    ReplaceWith: "[replaced]".to_string(),
                },
            },
        ]);
        let filter = PatternFilter::new(&cfg).unwrap();
        let input = "noise1\nreplace me\nkeep";
        let result = filter.apply(input);
        assert!(!result.contains("noise1"));
        assert!(result.contains("[replaced]"));
        assert!(result.contains("keep"));
    }
}
