use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CcrConfig {
    #[serde(default)]
    pub global: GlobalConfig,
    #[serde(default)]
    pub commands: HashMap<String, CommandConfig>,
    #[serde(default)]
    pub tee: TeeConfig,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct TeeConfig {
    pub enabled: bool,
    pub mode: TeeMode,
    pub max_files: usize,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum TeeMode {
    Aggressive,
    Always,
    Never,
}

impl Default for TeeConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: TeeMode::Aggressive,
            max_files: 20,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct GlobalConfig {
    pub summarize_threshold_lines: usize,
    pub head_lines: usize,
    pub tail_lines: usize,
    pub strip_ansi: bool,
    pub normalize_whitespace: bool,
    pub deduplicate_lines: bool,
}

impl Default for GlobalConfig {
    fn default() -> Self {
        Self {
            summarize_threshold_lines: 200,
            head_lines: 30,
            tail_lines: 30,
            strip_ansi: true,
            normalize_whitespace: true,
            deduplicate_lines: true,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CommandConfig {
    #[serde(default)]
    pub patterns: Vec<FilterPattern>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FilterPattern {
    pub regex: String,
    pub action: FilterAction,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum FilterAction {
    Simple(SimpleAction),
    ReplaceWith { ReplaceWith: String },
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq)]
pub enum SimpleAction {
    Remove,
    Collapse,
}
