use super::Handler;

pub struct DiffHandler;

impl Handler for DiffHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let mut out: Vec<String> = Vec::new();
        for line in output.lines() {
            if line.starts_with("+++")
                || line.starts_with("---")
                || line.starts_with("diff ")
                || line.starts_with("index ")
                || line.starts_with("@@")
                || line.starts_with('+')
                || line.starts_with('-')
            {
                out.push(line.to_string());
            }
            // Drop context lines (lines starting with space or empty)
        }
        if out.is_empty() {
            output.to_string()
        } else {
            out.join("\n")
        }
    }
}
