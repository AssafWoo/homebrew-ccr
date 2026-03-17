use super::Handler;

pub struct PythonHandler;

impl Handler for PythonHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let lines: Vec<&str> = output.lines().collect();

        if lines.len() <= 50 {
            return output.to_string();
        }

        // If there's a traceback, keep it + final error line; drop everything before
        if let Some(tb_pos) = output.find("Traceback (most recent call last):") {
            let tb_section = &output[tb_pos..];
            return tb_section.to_string();
        }

        // > 50 lines, no traceback: BERT summarization
        let result = ccr_core::summarizer::summarize(output, 40);
        result.output
    }
}
