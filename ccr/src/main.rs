use clap::{Parser, Subcommand};

mod cmd;
mod config_loader;
mod handlers;
mod hook;

#[derive(Parser)]
#[command(name = "ccr", about = "Cool Cost Reduction — LLM token optimizer")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Filter stdin to reduce token count
    Filter {
        /// Command hint for selecting filter rules (e.g. cargo, git, npm)
        #[arg(long)]
        command: Option<String>,
    },
    /// Show token savings analytics (per-command breakdown)
    Gain {
        /// Show per-day history instead of overall summary
        #[arg(long)]
        history: bool,
        /// Number of days to include in the history view
        #[arg(long, default_value = "14")]
        days: u32,
    },
    /// PostToolUse hook mode for Claude Code (hidden)
    #[command(hide = true)]
    Hook,
    /// Install CCR hooks into Claude Code settings.json
    Init,
    /// Execute a command through CCR's specialized handlers
    Run {
        /// The command and its arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Rewrite a command string for PreToolUse injection (hidden)
    #[command(hide = true)]
    Rewrite {
        /// Full command string to rewrite
        command: String,
    },
    /// Execute a command raw (no filtering) and record analytics
    Proxy {
        /// The command and its arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Scan Claude Code history and report missed optimization opportunities
    Discover,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Filter { command } => cmd::filter::run(command),
        Commands::Gain { history, days } => cmd::gain::run(history, days),
        Commands::Hook => hook::run(),
        Commands::Init => init(),
        Commands::Run { args } => cmd::run::run(args),
        Commands::Rewrite { command } => cmd::rewrite::run(command),
        Commands::Proxy { args } => cmd::proxy::run(args),
        Commands::Discover => cmd::discover::run(),
    };
    if let Err(e) = result {
        eprintln!("ccr error: {}", e);
        std::process::exit(1);
    }
}

fn init() -> anyhow::Result<()> {
    use serde_json::Value;

    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;

    let settings_path = home.join(".claude").join("settings.json");
    let hooks_dir = home.join(".claude").join("hooks");

    // Write ccr-rewrite.sh
    std::fs::create_dir_all(&hooks_dir)?;
    let rewrite_script_path = hooks_dir.join("ccr-rewrite.sh");
    let rewrite_script = r#"#!/usr/bin/env bash
INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
[ -z "$CMD" ] && exit 0
REWRITTEN=$(ccr rewrite "$CMD" 2>/dev/null) || exit 0
[ "$CMD" = "$REWRITTEN" ] && exit 0
ORIGINAL_INPUT=$(echo "$INPUT" | jq -c '.tool_input')
UPDATED_INPUT=$(echo "$ORIGINAL_INPUT" | jq --arg cmd "$REWRITTEN" '.command = $cmd')
jq -n --argjson updated "$UPDATED_INPUT" \
  '{"hookSpecificOutput":{"hookEventName":"PreToolUse","permissionDecision":"allow",
    "permissionDecisionReason":"CCR auto-rewrite","updatedInput":$updated}}'
"#;
    std::fs::write(&rewrite_script_path, rewrite_script)?;
    // chmod +x
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&rewrite_script_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&rewrite_script_path, perms)?;
    }

    // Load or create settings.json
    let mut settings: Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
    } else {
        serde_json::json!({})
    };

    // PostToolUse hook (existing)
    let post_hook_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [{ "type": "command", "command": "ccr hook" }]
    });

    // PreToolUse hook (new)
    let pre_hook_entry = serde_json::json!({
        "matcher": "Bash",
        "hooks": [{
            "type": "command",
            "command": rewrite_script_path.to_string_lossy()
        }]
    });

    settings["hooks"]["PostToolUse"] = serde_json::json!([post_hook_entry]);
    settings["hooks"]["PreToolUse"] = serde_json::json!([pre_hook_entry]);

    let parent = settings_path.parent().unwrap();
    std::fs::create_dir_all(parent)?;
    std::fs::write(&settings_path, serde_json::to_string_pretty(&settings)?)?;

    println!("CCR hooks installed:");
    println!("  PostToolUse: ccr hook → {}", settings_path.display());
    println!("  PreToolUse:  {} → {}", rewrite_script_path.display(), settings_path.display());
    Ok(())
}
