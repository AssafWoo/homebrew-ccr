pub mod cargo;
pub mod curl;
pub mod docker;
pub mod find;
pub mod git;
pub mod grep;
pub mod ls;
pub mod npm;
pub mod read;

/// A specialized handler for a known command.
/// Handlers may inject extra flags (`rewrite_args`) and compact the output (`filter`).
pub trait Handler: Send + Sync {
    /// Optionally rewrite the argument list before execution (e.g. inject --message-format json).
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        args.to_vec()
    }

    /// Filter the combined stdout+stderr output into a compact representation.
    fn filter(&self, output: &str, args: &[String]) -> String;
}

/// Returns a handler for the given command name, or `None` if the command is unknown.
pub fn get_handler(cmd: &str) -> Option<Box<dyn Handler>> {
    match cmd {
        "cargo" => Some(Box::new(cargo::CargoHandler)),
        "curl" => Some(Box::new(curl::CurlHandler)),
        "git" => Some(Box::new(git::GitHandler)),
        "docker" | "docker-compose" => Some(Box::new(docker::DockerHandler)),
        "npm" | "pnpm" | "yarn" => Some(Box::new(npm::NpmHandler)),
        "ls" => Some(Box::new(ls::LsHandler)),
        "cat" => Some(Box::new(read::ReadHandler)),
        "grep" | "rg" => Some(Box::new(grep::GrepHandler)),
        "find" => Some(Box::new(find::FindHandler)),
        _ => None,
    }
}

/// Returns true if `cmd` is a known handler command.
pub fn is_known(cmd: &str) -> bool {
    get_handler(cmd).is_some()
}
