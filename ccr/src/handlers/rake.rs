use super::Handler;

pub struct RakeHandler;

impl Handler for RakeHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        filter_minitest(output)
    }
}

/// Filter Minitest output (used by rake test / rake spec).
/// Keeps failure/error blocks and the final summary line; drops passing test lines.
fn filter_minitest(output: &str) -> String {
    let mut important: Vec<String> = Vec::new();
    let mut in_failure = false;
    let mut failure_lines = 0usize;

    for line in output.lines() {
        let t = line.trim();

        // Minitest failure/error block header: "1) Failure:" or "2) Error:"
        if !t.is_empty()
            && t.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false)
            && (t.contains("Failure:") || t.contains("Error:"))
        {
            in_failure = true;
            failure_lines = 0;
            important.push(line.to_string());
            continue;
        }

        if in_failure {
            important.push(line.to_string());
            failure_lines += 1;
            if (t.is_empty() && failure_lines > 2) || failure_lines >= 12 {
                in_failure = false;
            }
            continue;
        }

        // Summary: "N runs, N assertions, N failures, N errors, N skips"
        if t.contains("runs,") && t.contains("assertions,") {
            important.push(line.to_string());
        }

        // rake abort / exit with non-zero
        if t.starts_with("rake aborted!") || t.starts_with("Tasks:") {
            important.push(line.to_string());
        }
    }

    if important.is_empty() {
        output.to_string()
    } else {
        important.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_keeps_failure_block_and_summary() {
        let output = "\
Run options: --seed 12345

# Running:

..F.

Finished in 0.001s, 4000.0 runs/s, 4000.0 assertions/s.

1) Failure:
FooTest#test_bar [test/foo_test.rb:10]:
Expected false to be truthy.

4 runs, 4 assertions, 1 failures, 0 errors, 0 skips
";
        let result = filter_minitest(output);
        assert!(result.contains("Failure:"), "should keep failure block");
        assert!(result.contains("Expected false"), "should keep failure detail");
        assert!(result.contains("4 runs,"), "should keep summary");
        assert!(!result.contains("Run options:"), "should drop run options");
        assert!(!result.contains("# Running:"), "should drop running header");
    }

    #[test]
    fn filter_all_pass_returns_original() {
        let output = "...\n3 runs, 3 assertions, 0 failures, 0 errors, 0 skips\n";
        let result = filter_minitest(output);
        // With no failures, only the summary is kept
        assert!(result.contains("3 runs,"), "should keep summary");
    }
}
