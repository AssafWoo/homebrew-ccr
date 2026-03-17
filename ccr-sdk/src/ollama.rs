use anyhow::Result;
use serde::Deserialize;

pub struct OllamaConfig {
    /// Ollama base URL. Default: http://localhost:11434
    pub base_url: String,
    /// Model to use for summarization. Default: mistral:instruct
    pub model: String,
    /// Minimum cosine similarity between original and generated text to accept
    /// the generative output. Below this threshold we fall back to extractive.
    /// Default: 0.80
    pub similarity_threshold: f32,
}

impl Default for OllamaConfig {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: "mistral:instruct".to_string(),
            similarity_threshold: 0.80,
        }
    }
}

#[derive(Deserialize)]
struct GenerateResponse {
    response: String,
}

/// Returns true if Ollama is reachable and responding.
pub fn is_available(config: &OllamaConfig) -> bool {
    let url = format!("{}/api/tags", config.base_url);
    reqwest::blocking::Client::new()
        .get(&url)
        .timeout(std::time::Duration::from_secs(2))
        .send()
        .map(|r| r.status().is_success())
        .unwrap_or(false)
}

/// Ask Ollama to summarize `text`, preserving all specific facts.
pub fn summarize(text: &str, config: &OllamaConfig) -> Result<String> {
    let word_count = text.split_whitespace().count();
    let target_words = (word_count as f32 * 0.60).ceil() as usize;

    let prompt = format!(
        "Compress the following message to at most {target_words} words (60% of the original {word_count} words). \
         You MUST preserve every specific fact, number, name, constraint, and instruction — these cannot be dropped or paraphrased. \
         Aggressively cut filler words, repetition, elaboration, and anything that restates the obvious. \
         Be terse. Output only the compressed text, nothing else.\n\nMessage: {}",
        text
    );

    let url = format!("{}/api/generate", config.base_url);
    let body = serde_json::json!({
        "model": config.model,
        "prompt": prompt,
        "stream": false
    });

    let resp = reqwest::blocking::Client::new()
        .post(&url)
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()?
        .json::<GenerateResponse>()?;

    Ok(resp.response.trim().to_string())
}
