use super::Handler;

pub struct ReadHandler;

impl Handler for ReadHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();
        let n = lines.len();

        if n <= 100 {
            // Pass through short files
            return output.to_string();
        }

        if n <= 500 {
            // Head 60 + tail 20 with marker
            let head = &lines[..60];
            let tail = &lines[n.saturating_sub(20)..];
            let omitted = n - 60 - 20;
            let mut out: Vec<String> = head.iter().map(|l| l.to_string()).collect();
            out.push(format!("[... {} lines omitted ...]", omitted));
            out.extend(tail.iter().map(|l| l.to_string()));
            return out.join("\n");
        }

        // > 500 lines: use BERT semantic summarization
        let budget = 80; // keep ~80 lines
        let result = ccr_core::summarizer::summarize(output, budget);
        result.output
    }
}
