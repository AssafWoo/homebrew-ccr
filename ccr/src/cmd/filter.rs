use anyhow::Result;
use ccr_core::pipeline::Pipeline;
use std::io::{self, Read, Write};

pub fn run(command_hint: Option<String>) -> Result<()> {
    let config = crate::config_loader::load_config()?;
    let pipeline = Pipeline::new(config);

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;

    let result = pipeline.process(&input, command_hint.as_deref())?;

    io::stdout().write_all(result.output.as_bytes())?;

    // Append analytics to ~/.local/share/ccr/analytics.jsonl
    if let Some(data_dir) = dirs::data_local_dir() {
        let ccr_dir = data_dir.join("ccr");
        let _ = std::fs::create_dir_all(&ccr_dir);
        let analytics_path = ccr_dir.join("analytics.jsonl");
        if let Ok(json) = serde_json::to_string(&result.analytics) {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&analytics_path)
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "{}", json)
                });
        }
    }

    Ok(())
}
