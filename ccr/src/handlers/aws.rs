use super::Handler;

pub struct AwsHandler;

impl Handler for AwsHandler {
    fn rewrite_args(&self, args: &[String]) -> Vec<String> {
        if !args.iter().any(|a| a == "--output") {
            let mut out = args.to_vec();
            out.push("--output".to_string());
            out.push("json".to_string());
            return out;
        }
        args.to_vec()
    }

    fn filter(&self, output: &str, args: &[String]) -> String {
        let subcmd = args.get(1).map(|s| s.as_str()).unwrap_or("");
        let action = args.get(2).map(|s| s.as_str()).unwrap_or("");

        // Always preserve errors
        if output.trim_start().starts_with("An error") || output.contains("Error") && output.contains("Code") {
            return output.to_string();
        }

        if subcmd == "s3" && action == "ls" {
            return filter_s3_ls(output);
        }

        // JSON output: apply schema extraction
        let trimmed = output.trim();
        if trimmed.starts_with('{') || trimmed.starts_with('[') {
            if let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) {
                let schema = json_to_schema(&v);
                let schema_str = serde_json::to_string_pretty(&schema).unwrap_or_default();
                if schema_str.len() < trimmed.len() {
                    return schema_str;
                }
            }
        }

        output.to_string()
    }
}

fn filter_s3_ls(output: &str) -> String {
    // Group by prefix, show count + total size
    let mut prefixes: std::collections::HashMap<String, (usize, u64)> = std::collections::HashMap::new();
    let mut loose_count = 0usize;
    let mut loose_size = 0u64;

    for line in output.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        // Directory prefix: "                           PRE some-prefix/"
        if t.starts_with("PRE ") {
            let prefix = t[4..].trim().to_string();
            prefixes.entry(prefix).or_insert((0, 0));
            continue;
        }
        // File entry: "2023-01-01 00:00:00      12345 filename"
        let parts: Vec<&str> = t.split_whitespace().collect();
        if parts.len() >= 4 {
            if let Ok(size) = parts[2].parse::<u64>() {
                loose_count += 1;
                loose_size += size;
            }
        }
    }

    let mut out: Vec<String> = Vec::new();
    if loose_count > 0 {
        out.push(format!("{} objects, {} bytes", loose_count, loose_size));
    }
    for (prefix, (count, size)) in &prefixes {
        if *count > 0 {
            out.push(format!("{}: {} objects, {} bytes", prefix, count, size));
        } else {
            out.push(format!("{}/", prefix));
        }
    }
    if out.is_empty() {
        output.to_string()
    } else {
        out.sort();
        out.join("\n")
    }
}

fn json_to_schema(v: &serde_json::Value) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let schema_map: serde_json::Map<String, serde_json::Value> = map
                .iter()
                .map(|(k, val)| (k.clone(), json_to_schema(val)))
                .collect();
            serde_json::Value::Object(schema_map)
        }
        serde_json::Value::Array(arr) => {
            if arr.is_empty() {
                serde_json::json!(["array(0 items)"])
            } else {
                let first_schema = json_to_schema(&arr[0]);
                serde_json::json!([first_schema, format!("[{} items total]", arr.len())])
            }
        }
        serde_json::Value::String(_) => serde_json::Value::String("string".to_string()),
        serde_json::Value::Number(_) => serde_json::Value::String("number".to_string()),
        serde_json::Value::Bool(_) => serde_json::Value::String("boolean".to_string()),
        serde_json::Value::Null => serde_json::Value::String("null".to_string()),
    }
}
