use super::Handler;

pub struct TerraformHandler;

impl Handler for TerraformHandler {
    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        match subcmd {
            "plan" => filter_plan(output),
            "apply" => filter_apply(output),
            "init" => filter_init(output),
            "validate" => filter_validate(output),
            _ => output.to_string(),
        }
    }
}

fn filter_plan(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.starts_with('+')
            || t.starts_with('-')
            || t.starts_with('~')
            || t.starts_with("Plan:")
            || t.contains("No changes")
        {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_apply(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.contains(": Creating...")
            || t.contains(": Creation complete")
            || t.contains(": Destroying...")
            || t.contains(": Destruction complete")
            || t.contains(": Modifying...")
            || t.contains(": Modifications complete")
            || t.starts_with("Apply complete!")
            || t.contains("Error:")
            || t.starts_with("Error ")
        {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}

fn filter_init(output: &str) -> String {
    let has_error = output.lines().any(|l| {
        let t = l.trim();
        t.starts_with("Error") || t.starts_with("error")
    });
    if has_error {
        let errors: Vec<&str> = output
            .lines()
            .filter(|l| {
                let t = l.trim();
                t.starts_with("Error") || t.starts_with("error") || t.contains("Error:")
            })
            .collect();
        return errors.join("\n");
    }
    "[terraform init complete]".to_string()
}

fn filter_validate(output: &str) -> String {
    let mut out: Vec<String> = Vec::new();
    for line in output.lines() {
        let t = line.trim();
        if t.contains("Success") || t.contains("error") || t.contains("Error") || t.contains("warning") {
            out.push(line.to_string());
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.join("\n")
    }
}
