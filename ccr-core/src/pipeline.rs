use crate::analytics::Analytics;
use crate::ansi::strip_ansi;
use crate::config::CcrConfig;
use crate::patterns::PatternFilter;
use crate::summarizer::summarize;
use crate::tokens::count_tokens;
use crate::whitespace::normalize;

pub struct PipelineResult {
    pub output: String,
    pub analytics: Analytics,
}

pub struct Pipeline {
    pub config: CcrConfig,
}

impl Pipeline {
    pub fn new(config: CcrConfig) -> Self {
        Self { config }
    }

    pub fn process(&self, input: &str, command_hint: Option<&str>) -> anyhow::Result<PipelineResult> {
        let input_tokens = count_tokens(input);

        let mut text = input.to_string();

        // 1. Strip ANSI
        if self.config.global.strip_ansi {
            text = strip_ansi(&text);
        }

        // 2. Normalize whitespace
        if self.config.global.normalize_whitespace {
            text = normalize(&text, &self.config.global);
        }

        // 3. Apply command-specific patterns
        if let Some(hint) = command_hint {
            if let Some(cmd_config) = self.config.commands.get(hint) {
                let filter = PatternFilter::new(cmd_config)?;
                text = filter.apply(&text);
            }
        }

        // 4. Summarize if too long
        if text.lines().count() > self.config.global.summarize_threshold_lines {
            let budget = self.config.global.head_lines + self.config.global.tail_lines;
            text = summarize(&text, budget).output;
        }

        let output_tokens = count_tokens(&text);
        let analytics = Analytics::compute(input_tokens, output_tokens);

        Ok(PipelineResult {
            output: text,
            analytics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CcrConfig, CommandConfig, FilterAction, FilterPattern, SimpleAction};
    use std::collections::HashMap;

    fn default_pipeline() -> Pipeline {
        Pipeline::new(CcrConfig::default())
    }

    #[test]
    fn pipeline_strips_ansi_then_deduplicates() {
        let pipeline = default_pipeline();
        let input = "\x1b[32mgreen\x1b[0m\n\x1b[32mgreen\x1b[0m";
        let result = pipeline.process(input, None).unwrap();
        // After ANSI strip: "green\ngreen" -> after dedup: "green"
        assert_eq!(result.output.trim(), "green");
    }

    #[test]
    fn command_hint_selects_correct_patterns() {
        let mut commands = HashMap::new();
        commands.insert(
            "cargo".to_string(),
            CommandConfig {
                patterns: vec![FilterPattern {
                    regex: "^   Compiling \\S+ v[\\d.]+".to_string(),
                    action: FilterAction::Simple(SimpleAction::Collapse),
                }],
            },
        );
        let config = CcrConfig {
            commands,
            ..CcrConfig::default()
        };
        let pipeline = Pipeline::new(config);
        let input = "   Compiling foo v1.0\n   Compiling bar v1.0\nerror[E0001]: bad";
        let result = pipeline.process(input, Some("cargo")).unwrap();
        assert!(result.output.contains("collapsed") || result.output.contains("Compiling"));
        assert!(result.output.contains("error[E0001]"));
    }

    #[test]
    fn no_command_hint_uses_global_rules_only() {
        let mut commands = HashMap::new();
        commands.insert(
            "cargo".to_string(),
            CommandConfig {
                patterns: vec![FilterPattern {
                    regex: "^   Compiling \\S+ v[\\d.]+".to_string(),
                    action: FilterAction::Simple(SimpleAction::Remove),
                }],
            },
        );
        let config = CcrConfig {
            commands,
            ..CcrConfig::default()
        };
        let pipeline = Pipeline::new(config);
        let input = "   Compiling foo v1.0\n   Compiling bar v1.0";
        // No command hint: cargo patterns not applied
        let result = pipeline.process(input, None).unwrap();
        assert!(result.output.contains("Compiling"));
    }

    #[test]
    fn returns_correct_analytics() {
        let pipeline = default_pipeline();
        let input = "hello world";
        let result = pipeline.process(input, None).unwrap();
        assert!(result.analytics.input_tokens > 0);
        assert!(result.analytics.output_tokens > 0);
        assert!(result.analytics.savings_pct >= 0.0);
    }
}
