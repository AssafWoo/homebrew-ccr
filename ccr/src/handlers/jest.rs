use super::Handler;
use super::util;

pub struct JestHandler;

impl Handler for JestHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        util::test_failures(output, "jest")
    }
}
