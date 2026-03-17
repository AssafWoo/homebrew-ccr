use super::Handler;
use super::util;

pub struct VitestHandler;

impl Handler for VitestHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        if !args.iter().any(|a| a == "--reporter") {
            let mut out = args.to_vec();
            out.push("--reporter=verbose".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, _args: &[String]) -> String {
        util::test_failures(output, "vitest")
    }
}
