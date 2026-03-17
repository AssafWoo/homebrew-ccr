use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::io::{self, Read};

#[derive(Debug, Deserialize)]
struct HookInput {
    #[serde(default)]
    tool_name: String,
    #[serde(default)]
    tool_input: serde_json::Value,
    #[serde(default)]
    tool_response: ToolResponse,
}

#[derive(Debug, Deserialize, Default)]
struct ToolResponse {
    #[serde(default)]
    output: String,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct HookOutput {
    output: String,
}

pub fn run() -> Result<()> {
    let mut input = String::new();
    if let Err(_) = io::stdin().read_to_string(&mut input) {
        // Never block Claude Code
        return Ok(());
    }

    let hook_input: HookInput = match serde_json::from_str(&input) {
        Ok(v) => v,
        Err(_) => {
            // malformed JSON — pass through silently, never block
            return Ok(());
        }
    };

    // Determine command hint from tool_input.command
    let command_hint = hook_input
        .tool_input
        .get("command")
        .and_then(|v| v.as_str())
        .and_then(|cmd| {
            // Extract first word as command hint
            let first = cmd.split_whitespace().next()?;
            Some(first.to_string())
        });

    let output_text = if let Some(err) = &hook_input.tool_response.error {
        err.clone()
    } else {
        hook_input.tool_response.output.clone()
    };

    if output_text.is_empty() {
        return Ok(());
    }

    let config = match crate::config_loader::load_config() {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let pipeline = ccr_core::pipeline::Pipeline::new(config);
    let result = match pipeline.process(&output_text, command_hint.as_deref()) {
        Ok(r) => r,
        Err(_) => return Ok(()),
    };

    // Output the filtered text as JSON for Claude Code to use
    let hook_output = HookOutput {
        output: result.output,
    };
    println!("{}", serde_json::to_string(&hook_output)?);

    Ok(())
}
