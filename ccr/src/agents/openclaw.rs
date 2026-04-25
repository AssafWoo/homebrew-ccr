//! OpenClaw agent installer.
//!
//! OpenClaw stores its hook configuration at `~/.openclaw/openclaw.json`.
//!
//! Hook registration format (flat string arrays per OpenClaw docs):
//!   ```json
//!   { "hooks": {
//!       "preToolUse":  ["/path/to/panda-rewrite.sh"],
//!       "postToolUse": ["PANDA_SESSION_ID=$PPID PANDA_AGENT=openclaw /path/to/panda hook"]
//!   }}
//!   ```
//!
//! Hook input (preToolUse):  stdin JSON with `tool_name`, `tool_input.command`
//! Hook output (rewrite): `{"decision": "allow", "hookSpecificOutput": {"tool_input": {"command": "..."}}}`
//! Hook output (no-op):   `{"decision": "allow"}`
//!
//! Exit 0 on ALL error paths — OpenClaw terminates on non-zero hook exit.

use super::AgentInstaller;
use std::path::PathBuf;

pub struct OpenClawInstaller;

fn openclaw_dir() -> Option<PathBuf> {
    Some(dirs::home_dir()?.join(".openclaw"))
}

fn config_path() -> Option<PathBuf> {
    Some(openclaw_dir()?.join("openclaw.json"))
}

fn hooks_dir() -> Option<PathBuf> {
    Some(openclaw_dir()?.join("hooks"))
}

impl AgentInstaller for OpenClawInstaller {
    fn name(&self) -> &'static str {
        "OpenClaw"
    }

    fn install(&self, panda_bin: &str) -> anyhow::Result<()> {
        let Some(openclaw_dir) = openclaw_dir() else {
            anyhow::bail!("Cannot determine OpenClaw config directory");
        };

        // Only install if OpenClaw is already present on this machine.
        if !openclaw_dir.exists() {
            println!("OpenClaw not found (no ~/.openclaw directory) — skipping OpenClaw install.");
            println!("If you install OpenClaw later, run: panda init --agent openclaw");
            return Ok(());
        }

        let Some(hooks_dir) = hooks_dir() else {
            anyhow::bail!("Cannot determine OpenClaw hooks directory");
        };
        std::fs::create_dir_all(&hooks_dir)?;

        // Write PreToolUse rewrite script
        let script_path = hooks_dir.join("panda-rewrite.sh");
        let script = generate_openclaw_rewrite_script(panda_bin);
        std::fs::write(&script_path, &script)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script_path, perms)?;
        }

        // Write integrity baseline
        if let Err(e) = crate::integrity::write_baseline(&script_path, &hooks_dir) {
            eprintln!("warning: could not write integrity baseline: {e}");
        }

        // Load or create openclaw.json
        let Some(config_path) = config_path() else {
            anyhow::bail!("Cannot determine OpenClaw config path");
        };

        let mut root: serde_json::Value = if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            serde_json::from_str(&content).unwrap_or(serde_json::json!({}))
        } else {
            serde_json::json!({})
        };

        let script_str = script_path.to_string_lossy().to_string();
        let hook_cmd = format!(
            "PANDA_SESSION_ID=$PPID PANDA_AGENT=openclaw {} hook",
            panda_bin
        );

        // Remove any existing PandaFilter entries before re-inserting
        remove_panda_entries(&mut root, "preToolUse");
        remove_panda_entries(&mut root, "postToolUse");

        // Insert preToolUse rewrite script
        insert_hook_entry(&mut root, "preToolUse", &script_str);
        // Insert postToolUse compression command
        insert_hook_entry(&mut root, "postToolUse", &hook_cmd);

        std::fs::write(&config_path, serde_json::to_string_pretty(&root)?)?;

        println!("PandaFilter hooks installed (OpenClaw):");
        println!("  Rewrite script: {}", script_path.display());
        println!("  Config:         {}", config_path.display());

        Ok(())
    }

    fn uninstall(&self) -> anyhow::Result<()> {
        let Some(openclaw_dir) = openclaw_dir() else {
            return Ok(());
        };

        // Remove rewrite script
        let script_path = openclaw_dir.join("hooks").join("panda-rewrite.sh");
        if script_path.exists() {
            std::fs::remove_file(&script_path)?;
            println!("Removed {}", script_path.display());
        }

        // Remove integrity baseline
        let hash_path = openclaw_dir.join("hooks").join(".panda-hook.sha256");
        if hash_path.exists() {
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                if let Ok(meta) = std::fs::metadata(&hash_path) {
                    let mut perms = meta.permissions();
                    perms.set_mode(0o644);
                    let _ = std::fs::set_permissions(&hash_path, perms);
                }
            }
            std::fs::remove_file(&hash_path)?;
            println!("Removed {}", hash_path.display());
        }

        // Strip PandaFilter entries from openclaw.json
        let Some(config_path) = config_path() else {
            return Ok(());
        };
        if config_path.exists() {
            let content = std::fs::read_to_string(&config_path)?;
            let mut root: serde_json::Value =
                serde_json::from_str(&content).unwrap_or(serde_json::json!({}));

            remove_panda_entries(&mut root, "preToolUse");
            remove_panda_entries(&mut root, "postToolUse");

            std::fs::write(&config_path, serde_json::to_string_pretty(&root)?)?;
            println!("Removed PandaFilter entries from {}", config_path.display());
        }

        Ok(())
    }
}

/// Generate the OpenClaw PreToolUse shell script that rewrites commands.
/// Always exits 0 — OpenClaw terminates on non-zero hook exit.
fn generate_openclaw_rewrite_script(panda_bin: &str) -> String {
    format!(
        r#"#!/usr/bin/env bash
# PandaFilter OpenClaw preToolUse hook
# Rewrites shell invocations for token savings.
# ALWAYS exits 0 — OpenClaw terminates on non-zero hook exit.
INPUT=$(cat)
CMD=$(echo "$INPUT" | jq -r '.tool_input.command // empty')
if [ -z "$CMD" ]; then
  echo '{{"decision": "allow"}}'
  exit 0
fi
REWRITTEN=$(PANDA_SESSION_ID=$PPID "{panda_bin}" rewrite "$CMD" 2>/dev/null) || {{
  echo '{{"decision": "allow"}}'
  exit 0
}}
if [ "$CMD" = "$REWRITTEN" ]; then
  echo '{{"decision": "allow"}}'
  exit 0
fi
jq -n --arg cmd "$REWRITTEN" '{{
  "decision": "allow",
  "hookSpecificOutput": {{
    "tool_input": {{"command": $cmd}}
  }}
}}'
"#,
        panda_bin = panda_bin
    )
}

/// Remove all PandaFilter command entries from `hooks.<event>` in `root`.
fn remove_panda_entries(root: &mut serde_json::Value, event: &str) {
    if let Some(arr) = root
        .get_mut("hooks")
        .and_then(|h| h.get_mut(event))
        .and_then(|e| e.as_array_mut())
    {
        arr.retain(|entry| {
            let cmd = entry.as_str().unwrap_or("");
            !cmd.contains("panda") && !cmd.contains("ccr")
        });
    }
}

/// Insert a command string into `hooks.<event>` if not already present.
fn insert_hook_entry(root: &mut serde_json::Value, event: &str, command: &str) {
    let root_obj = root.as_object_mut().unwrap();
    let hooks = root_obj
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .unwrap();
    let arr = hooks
        .entry(event)
        .or_insert_with(|| serde_json::json!([]))
        .as_array_mut()
        .unwrap();

    let already = arr.iter().any(|e| e.as_str().unwrap_or("") == command);
    if !already {
        arr.push(serde_json::json!(command));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_and_remove_entries() {
        let mut root = serde_json::json!({});
        insert_hook_entry(&mut root, "preToolUse", "/usr/local/bin/panda-rewrite.sh");
        insert_hook_entry(
            &mut root,
            "postToolUse",
            "PANDA_AGENT=openclaw /usr/local/bin/panda hook",
        );

        assert_eq!(root["hooks"]["preToolUse"].as_array().unwrap().len(), 1);
        assert_eq!(root["hooks"]["postToolUse"].as_array().unwrap().len(), 1);

        // Inserting same command again is a no-op
        insert_hook_entry(&mut root, "preToolUse", "/usr/local/bin/panda-rewrite.sh");
        assert_eq!(root["hooks"]["preToolUse"].as_array().unwrap().len(), 1);

        remove_panda_entries(&mut root, "preToolUse");
        remove_panda_entries(&mut root, "postToolUse");

        assert!(root["hooks"]["preToolUse"].as_array().unwrap().is_empty());
        assert!(root["hooks"]["postToolUse"].as_array().unwrap().is_empty());
    }

    #[test]
    fn remove_preserves_non_panda_entries() {
        let mut root = serde_json::json!({
            "hooks": {
                "preToolUse": [
                    "/usr/bin/other-hook.sh",
                    "/usr/local/bin/panda-rewrite.sh"
                ]
            }
        });

        remove_panda_entries(&mut root, "preToolUse");
        let arr = root["hooks"]["preToolUse"].as_array().unwrap();
        assert_eq!(arr.len(), 1);
        assert!(arr[0].as_str().unwrap().contains("other-hook"));
    }

    #[test]
    fn script_contains_jq_rewrite() {
        let script = generate_openclaw_rewrite_script("/usr/local/bin/panda");
        assert!(script.contains("/usr/local/bin/panda"));
        assert!(script.contains("rewrite"));
        assert!(script.contains("hookSpecificOutput"));
        assert!(script.contains("tool_input"));
    }
}
