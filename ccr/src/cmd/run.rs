use anyhow::Result;
use ccr_core::tokens;
use std::path::PathBuf;
use std::process::Command;

pub fn run(args: Vec<String>) -> Result<()> {
    if args.is_empty() {
        anyhow::bail!("ccr run: no command specified");
    }

    let cmd_name = args[0].clone();
    // Extract subcommand: first non-flag argument after argv[0]
    let subcommand = args
        .get(1)
        .filter(|s| !s.starts_with('-'))
        .cloned();

    let handler = crate::handlers::get_handler(&cmd_name);

    // Rewrite args (e.g. inject --message-format json for cargo)
    let final_args: Vec<String> = if let Some(ref h) = handler {
        h.rewrite_args(&args)
    } else {
        args.clone()
    };

    // Execute the command, capturing stdout+stderr
    let t0 = std::time::Instant::now();
    let output = Command::new(&final_args[0])
        .args(&final_args[1..])
        .output()
        .map_err(|e| anyhow::anyhow!("failed to execute '{}': {}", cmd_name, e))?;
    let duration_ms = t0.elapsed().as_millis() as u64;

    let raw_output = {
        let mut combined = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        if !stderr.is_empty() {
            if !combined.is_empty() {
                combined.push('\n');
            }
            combined.push_str(&stderr);
        }
        combined
    };

    // Tee: save raw output to disk
    let tee_path = write_tee(&cmd_name, &raw_output);

    // Filter the output
    let filtered = if let Some(ref h) = handler {
        h.filter(&raw_output, &args)
    } else {
        // Pipeline fallback for unknown commands
        let config = match crate::config_loader::load_config() {
            Ok(c) => c,
            Err(_) => ccr_core::config::CcrConfig::default(),
        };
        let pipeline = ccr_core::pipeline::Pipeline::new(config);
        match pipeline.process(&raw_output, Some(&cmd_name)) {
            Ok(r) => r.output,
            Err(_) => raw_output.clone(),
        }
    };

    // Compute analytics
    let input_tokens = tokens::count_tokens(&raw_output);
    let output_tokens = tokens::count_tokens(&filtered);
    let savings_pct = if input_tokens == 0 {
        0.0
    } else {
        let saved = input_tokens.saturating_sub(output_tokens);
        (saved as f32 / input_tokens as f32) * 100.0
    };

    // Append tee hint if compression is aggressive (>60%) and tee was written
    let mut display_output = filtered.clone();
    if savings_pct > 60.0 {
        if let Some(ref path) = tee_path {
            display_output.push_str(&format!("\n[full output: {}]", path.display()));
        }
    }

    print!("{}", display_output);
    // Ensure trailing newline
    if !display_output.ends_with('\n') {
        println!();
    }

    // Record analytics
    let analytics = ccr_core::analytics::Analytics::new(
        input_tokens,
        output_tokens,
        Some(cmd_name),
        subcommand,
        Some(duration_ms),
    );
    append_analytics(&analytics);

    // Propagate exit code
    let code = output.status.code().unwrap_or(1);
    if code != 0 {
        std::process::exit(code);
    }

    Ok(())
}

fn write_tee(cmd: &str, content: &str) -> Option<PathBuf> {
    let tee_dir = dirs::data_local_dir()?.join("ccr").join("tee");
    std::fs::create_dir_all(&tee_dir).ok()?;

    // Rotate: keep max 20 files
    if let Ok(entries) = std::fs::read_dir(&tee_dir) {
        let mut log_files: Vec<(std::time::SystemTime, PathBuf)> = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .ends_with(".log")
            })
            .filter_map(|e| {
                let modified = e.metadata().ok()?.modified().ok()?;
                Some((modified, e.path()))
            })
            .collect();

        if log_files.len() >= 20 {
            log_files.sort_by_key(|(t, _)| *t);
            for (_, path) in log_files.iter().take(log_files.len() - 19) {
                let _ = std::fs::remove_file(path);
            }
        }
    }

    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let safe_cmd = cmd
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect::<String>();

    let path = tee_dir.join(format!("{}_{}.log", ts, safe_cmd));
    std::fs::write(&path, content).ok()?;
    Some(path)
}

fn append_analytics(analytics: &ccr_core::analytics::Analytics) {
    if let Some(data_dir) = dirs::data_local_dir() {
        let ccr_dir = data_dir.join("ccr");
        let _ = std::fs::create_dir_all(&ccr_dir);
        let path = ccr_dir.join("analytics.jsonl");
        if let Ok(json) = serde_json::to_string(analytics) {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{}", json)
                });
        }
    }
}
