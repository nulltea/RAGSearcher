use anyhow::Result;
use std::sync::OnceLock;
use tokenizers::Tokenizer;

const MODEL_NAME: &str = "google/embeddinggemma-300m";

static EMBEDDING_TOKENIZER: OnceLock<Result<Tokenizer, String>> = OnceLock::new();

pub fn embedding_tokenizer() -> Result<&'static Tokenizer> {
    EMBEDDING_TOKENIZER
        .get_or_init(|| {
        Tokenizer::from_pretrained(MODEL_NAME, None)
                .map_err(|e| format!("failed to load {MODEL_NAME} tokenizer: {e}"))
        })
        .as_ref()
        .map_err(|message| anyhow::anyhow!(message.clone()))
}

pub fn count_embedding_tokens(text: &str) -> Result<usize> {
    let tokenizer = embedding_tokenizer()?;
    tokenizer
        .encode(text, false)
        .map(|encoding| encoding.len())
        .map_err(|e| anyhow::anyhow!("failed to tokenize text: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_tokens_for_non_empty_text() {
        let count = count_embedding_tokens("A short retrieval chunk.").unwrap();
        assert!(count > 0);
    }
}
