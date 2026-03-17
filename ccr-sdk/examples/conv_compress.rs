/// Demonstration of conversation compression.
///
/// Run with:
///   cargo run -p ccr-sdk --example conv_compress
///
/// Shows token savings and which sentences survived for each user turn.
use ccr_core::tokens::count_tokens;
use ccr_sdk::compressor::CompressionConfig;
use ccr_sdk::message::Message;
use ccr_sdk::ollama::OllamaConfig;
use ccr_sdk::optimizer::Optimizer;

fn main() {
    let conversation = vec![
        Message {
            role: "user".into(),
            content: "Hey, I want to build a Rust CLI tool. The tool should reduce token usage when working with LLM APIs. It needs to hook into Claude Code. I want to make sure we don't lose any important error messages. The config should be in TOML. I want it to be fast and lightweight.".into(),
        },
        Message {
            role: "assistant".into(),
            content: "Got it. I'll build a Rust workspace with a core library, a CLI binary, and a hook that integrates with Claude Code's PostToolUse event. The config will use TOML, and we'll ensure errors are always preserved.".into(),
        },
        Message {
            role: "user".into(),
            content: "Great. Now I also want to compress the conversation history itself, not just command output. The idea is to use BERT embeddings to find semantically important sentences and keep those. Assistant messages should always be kept verbatim. We should never drop question sentences from user messages.".into(),
        },
        Message {
            role: "assistant".into(),
            content: "Understood. I'll implement sentence-level extractive summarization using AllMiniLML6V2. The algorithm scores sentences by distance from the centroid — outliers carry unique meaning and are kept. Questions and code-bearing sentences are hard-kept.".into(),
        },
        Message {
            role: "user".into(),
            content: "I think we should have two tiers. Recent messages stay untouched. Middle-age messages get light compression, maybe 55% of sentences. Older messages get aggressive compression, around 20%. What do you think about that approach?".into(),
        },
        Message {
            role: "assistant".into(),
            content: "Two-tier is a good balance. Recent context stays sharp, old context gets distilled to its core constraints. I'll implement CompressionConfig with recent_n=3, tier1_n=5, tier1_ratio=0.55, tier2_ratio=0.20 as defaults.".into(),
        },
        Message {
            role: "user".into(),
            content: "Also, I want analytics. Every compression should track tokens in and tokens out so we can show the user how much they saved. This is important for the product story. Make sure the analytics are persisted to disk just like the command output analytics.".into(),
        },
        Message {
            role: "assistant".into(),
            content: "I'll add tokens_in and tokens_out to CompressResult. The Optimizer will return this alongside the compressed messages so callers can log savings.".into(),
        },
        Message {
            role: "user".into(),
            content: "And in v2 we should explore using a local generative model like Ollama to paraphrase old messages before compression. But for now MVP is the priority. Let's ship the extractive BERT approach first and measure recall with the eval framework.".into(),
        },
        Message {
            role: "assistant".into(),
            content: "Agreed. Extractive first, measure with ccr-eval, then layer in generative for tier 2 once we have recall baselines.".into(),
        },
        Message {
            role: "user".into(),
            content: "Perfect. Can you now show me the compression in action on a real conversation? I want to see which sentences survive and what the token savings look like across all tiers.".into(),
        },
    ];

    let total_tokens_before: usize = conversation.iter().map(|m| count_tokens(&m.content)).sum();

    println!("CCR SDK — Conversation Compression Demo");
    println!("========================================");
    println!("Turns: {}  |  Tokens before: {}", conversation.len(), total_tokens_before);
    println!();

    let optimizer = Optimizer {
        config: CompressionConfig {
            ollama: Some(OllamaConfig::default()),
            ..CompressionConfig::default()
        },
    };
    let result = optimizer.compress(conversation.clone());

    let total = conversation.len();
    for (i, (before, after)) in conversation.iter().zip(result.messages.iter()).enumerate() {
        let age = total - 1 - i;
        let tier = if before.role == "assistant" {
            "verbatim (assistant)"
        } else if age < 3 {
            "verbatim (recent)"
        } else if age < 8 {
            "tier 1 (light)"
        } else {
            "tier 2 (aggressive)"
        };

        let t_in = count_tokens(&before.content);
        let t_out = count_tokens(&after.content);
        let changed = before.content != after.content;

        println!("Turn {} [{}] [{}]", i + 1, before.role.to_uppercase(), tier);

        if changed {
            println!("  BEFORE ({} tok): {}", t_in, before.content);
            println!("  AFTER  ({} tok): {}", t_out, after.content);
            println!("  Saved: {} tokens ({:.0}%)", t_in - t_out, (t_in - t_out) as f32 / t_in as f32 * 100.0);
        } else {
            println!("  KEPT   ({} tok): {}", t_in, truncate(&after.content, 100));
        }
        println!();
    }

    println!("========================================");
    println!("TOTAL:  {} → {} tokens  ({:.1}% saved)",
        result.tokens_in,
        result.tokens_out,
        (result.tokens_in - result.tokens_out) as f32 / result.tokens_in as f32 * 100.0,
    );
}

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max { s.to_string() } else { format!("{}…", &s[..max]) }
}
