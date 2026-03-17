use super::Handler;

pub struct EnvHandler;

const SENSITIVE_PATTERNS: &[&str] = &[
    "KEY", "SECRET", "TOKEN", "PASSWORD", "PASS", "CREDENTIAL", "AUTH",
];

impl Handler for EnvHandler {
    fn filter(&self, output: &str, _args: &[String]) -> String {
        let mut vars: Vec<(String, String)> = output
            .lines()
            .filter_map(|line| {
                let eq = line.find('=')?;
                let key = line[..eq].to_string();
                let val = line[eq + 1..].to_string();
                Some((key, val))
            })
            .map(|(k, v)| {
                let k_upper = k.to_uppercase();
                let is_sensitive = SENSITIVE_PATTERNS
                    .iter()
                    .any(|pat| k_upper.contains(pat));
                let v_out = if is_sensitive {
                    "[redacted]".to_string()
                } else {
                    v
                };
                (k, v_out)
            })
            .collect();

        vars.sort_by(|a, b| a.0.cmp(&b.0));

        const MAX_VARS: usize = 40;
        let total = vars.len();
        let shown = vars.iter().take(MAX_VARS);
        let mut out: Vec<String> = shown.map(|(k, v)| format!("{}={}", k, v)).collect();
        if total > MAX_VARS {
            out.push(format!("[+{} more env vars]", total - MAX_VARS));
        }
        out.join("\n")
    }
}
