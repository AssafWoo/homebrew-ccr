use super::Handler;
use super::util;

pub struct PytestHandler;

impl Handler for PytestHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        let has_verbosity = args.iter().any(|a| a == "-v" || a == "-s" || a == "-q");
        if !has_verbosity {
            let mut out = args.to_vec();
            out.push("-q".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, _args: &[String]) -> String {
        util::test_failures(output, "pytest")
    }
}
