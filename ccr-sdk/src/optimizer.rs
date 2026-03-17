use crate::compressor::{compress, CompressionConfig, CompressResult};
use crate::deduplicator::deduplicate;
use crate::message::Message;

pub struct Optimizer {
    pub config: CompressionConfig,
}

impl Default for Optimizer {
    fn default() -> Self {
        Self {
            config: CompressionConfig::default(),
        }
    }
}

impl Optimizer {
    pub fn compress(&self, messages: Vec<Message>) -> CompressResult {
        let deduped = deduplicate(messages);
        compress(deduped, &self.config)
    }
}
