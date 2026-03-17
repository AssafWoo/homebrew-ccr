pub mod aws;
pub mod cargo;
pub mod curl;
pub mod diff;
pub mod docker;
pub mod env;
pub mod eslint;
pub mod find;
pub mod gh;
pub mod git;
pub mod grep;
pub mod jest;
pub mod jq;
pub mod kubectl;
pub mod ls;
pub mod make;
pub mod npm;
pub mod pip;
pub mod psql;
pub mod pytest;
pub mod python;
pub mod read;
pub mod terraform;
pub mod tree;
pub mod tsc;
pub mod util;
pub mod vitest;

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
        // Existing handlers
        "cargo" => Some(Box::new(cargo::CargoHandler)),
        "curl" => Some(Box::new(curl::CurlHandler)),
        "git" => Some(Box::new(git::GitHandler)),
        "docker" | "docker-compose" => Some(Box::new(docker::DockerHandler)),
        "npm" | "pnpm" | "yarn" => Some(Box::new(npm::NpmHandler)),
        "ls" => Some(Box::new(ls::LsHandler)),
        "cat" => Some(Box::new(read::ReadHandler)),
        "grep" | "rg" => Some(Box::new(grep::GrepHandler)),
        "find" => Some(Box::new(find::FindHandler)),
        // Batch 1: TypeScript / JavaScript
        "tsc" => Some(Box::new(tsc::TscHandler)),
        "vitest" => Some(Box::new(vitest::VitestHandler)),
        "jest" => Some(Box::new(jest::JestHandler)),
        "eslint" => Some(Box::new(eslint::EslintHandler)),
        // Batch 2: Python
        "pytest" => Some(Box::new(pytest::PytestHandler)),
        "pip" | "pip3" | "uv" => Some(Box::new(pip::PipHandler)),
        "python" | "python3" => Some(Box::new(python::PythonHandler)),
        // Batch 3: DevOps / Cloud
        "kubectl" => Some(Box::new(kubectl::KubectlHandler)),
        "gh" => Some(Box::new(gh::GhHandler)),
        "terraform" | "tofu" => Some(Box::new(terraform::TerraformHandler)),
        "aws" => Some(Box::new(aws::AwsHandler)),
        "make" | "gmake" => Some(Box::new(make::MakeHandler)),
        // Batch 4: System / Utility
        "psql" | "pgcli" => Some(Box::new(psql::PsqlHandler)),
        "tree" => Some(Box::new(tree::TreeHandler)),
        "diff" => Some(Box::new(diff::DiffHandler)),
        "jq" => Some(Box::new(jq::JqHandler)),
        "env" | "printenv" => Some(Box::new(env::EnvHandler)),
        _ => None,
    }
}

/// Returns true if `cmd` is a known handler command.
pub fn is_known(cmd: &str) -> bool {
    get_handler(cmd).is_some()
}
