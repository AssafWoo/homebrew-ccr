use anyhow::Result;
use crate::integrity::{verify_hook, IntegrityStatus};

pub fn run() -> Result<()> {
    let home = dirs::home_dir()
        .ok_or_else(|| anyhow::anyhow!("Cannot find home directory"))?;
    let script  = home.join(".claude").join("hooks").join("ccr-rewrite.sh");
    let hashdir = home.join(".claude").join("hooks");
    let hash_file = hashdir.join(".ccr-hook.sha256");

    match verify_hook(&script, &hashdir) {
        IntegrityStatus::Verified => {
            println!("OK  Verified   {}", script.display());
        }
        IntegrityStatus::Tampered { expected, actual } => {
            println!("ERR Tampered   {}", script.display());
            println!("    Expected: {}", expected);
            println!("    Actual:   {}", actual);
            println!();
            println!("Run `ccr init` to reinstall the hook and reset the baseline.");
            std::process::exit(1);
        }
        IntegrityStatus::NoBaseline => {
            println!("?   No baseline — hash file not found: {}", hash_file.display());
            println!("    Run `ccr init` to create the baseline.");
        }
        IntegrityStatus::NotInstalled => {
            println!("-   Not installed (neither hook script nor hash file found)");
            println!("    Run `ccr init` to install.");
        }
        IntegrityStatus::OrphanedHash => {
            println!("?   Orphaned hash — script missing, hash exists: {}", hash_file.display());
            println!("    Run `ccr init` to reinstall.");
        }
    }
    Ok(())
}
