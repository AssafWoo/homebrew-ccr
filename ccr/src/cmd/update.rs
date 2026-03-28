use anyhow::{Result, bail};
use std::process::Command;

const REPO: &str = "AssafWoo/homebrew-ccr";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn run() -> Result<()> {
    println!("Checking for updates (current: v{})...", CURRENT_VERSION);

    let latest_tag = fetch_latest_tag()?;
    let latest = latest_tag.trim_start_matches('v');

    if !version_gt(latest, CURRENT_VERSION) {
        println!("Already up to date (v{}).", CURRENT_VERSION);
        return Ok(());
    }

    println!("Update available: v{} → v{}", CURRENT_VERSION, latest);

    let asset = platform_asset()?;
    let url = format!(
        "https://github.com/{}/releases/download/{}/{}",
        REPO, latest_tag, asset
    );

    let install_path = std::env::current_exe()
        .map_err(|e| anyhow::anyhow!("Cannot determine binary path: {}", e))?;

    let tmp_path = install_path.with_extension("tmp");

    println!("Downloading {}...", asset);
    let status = Command::new("curl")
        .args(["-fsSL", &url, "-o", tmp_path.to_str().unwrap_or("ccr.tmp")])
        .status()
        .map_err(|e| anyhow::anyhow!("curl not found: {}", e))?;

    if !status.success() {
        bail!("Download failed. Check your internet connection or visit:\nhttps://github.com/{}/releases/latest", REPO);
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&tmp_path, std::fs::Permissions::from_mode(0o755))
            .map_err(|e| anyhow::anyhow!("Cannot set permissions: {}", e))?;
    }

    std::fs::rename(&tmp_path, &install_path)
        .map_err(|e| anyhow::anyhow!("Cannot replace binary at {}: {}", install_path.display(), e))?;

    println!("Updated to v{}.", latest);
    println!("Re-registering hooks...");

    let init_status = Command::new(&install_path).arg("init").status();
    match init_status {
        Ok(s) if s.success() => println!("Done."),
        _ => println!("Warning: hook re-registration failed — run `ccr init` manually."),
    }

    Ok(())
}

fn fetch_latest_tag() -> Result<String> {
    let api_url = format!("https://api.github.com/repos/{}/releases/latest", REPO);
    let output = Command::new("curl")
        .args(["-fsSL", "-H", "Accept: application/vnd.github+json", &api_url])
        .output()
        .map_err(|e| anyhow::anyhow!("curl not found: {}", e))?;

    if !output.status.success() {
        bail!("Failed to fetch release info from GitHub");
    }

    let body = String::from_utf8_lossy(&output.stdout);
    let json: serde_json::Value = serde_json::from_str(&body)
        .map_err(|_| anyhow::anyhow!("Could not parse GitHub API response"))?;
    json.get("tag_name")
        .and_then(|t| t.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| anyhow::anyhow!("No tag_name in GitHub API response"))
}

/// Returns true if `a` is strictly greater than `b` (simple x.y.z comparison).
fn version_gt(a: &str, b: &str) -> bool {
    let parse = |v: &str| -> (u32, u32, u32) {
        let mut parts = v.splitn(3, '.');
        let major = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let minor = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        let patch = parts.next().and_then(|s| s.parse().ok()).unwrap_or(0);
        (major, minor, patch)
    };
    parse(a) > parse(b)
}

fn platform_asset() -> Result<&'static str> {
    match (std::env::consts::OS, std::env::consts::ARCH) {
        ("macos", "aarch64") => Ok("ccr-macos-arm64"),
        ("macos", "x86_64")  => Ok("ccr-macos-x86_64"),
        ("linux", "x86_64")  => Ok("ccr-linux-x86_64"),
        (os, arch) => bail!("No pre-built binary for {}/{}. Build from source: cargo build --release", os, arch),
    }
}
