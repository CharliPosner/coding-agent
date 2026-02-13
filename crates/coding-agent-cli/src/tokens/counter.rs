//! Token counting using tiktoken-rs.
//!
//! Provides accurate token counting for Claude and other models
//! using the cl100k_base tokenizer (compatible with Claude's tokenization).

use std::sync::OnceLock;
use thiserror::Error;
use tiktoken_rs::CoreBPE;

/// Errors that can occur during token counting.
#[derive(Debug, Error)]
pub enum TokenCounterError {
    /// Failed to initialize the tokenizer.
    #[error("failed to initialize tokenizer: {0}")]
    InitError(String),
}

/// Result of counting tokens in a text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenCount {
    /// Number of tokens in the text.
    pub tokens: usize,
}

impl TokenCount {
    /// Create a new token count.
    pub fn new(tokens: usize) -> Self {
        Self { tokens }
    }

    /// Create a zero token count.
    pub fn zero() -> Self {
        Self { tokens: 0 }
    }
}

impl std::ops::Add for TokenCount {
    type Output = Self;

    fn add(self, other: Self) -> Self {
        Self {
            tokens: self.tokens + other.tokens,
        }
    }
}

impl std::ops::AddAssign for TokenCount {
    fn add_assign(&mut self, other: Self) {
        self.tokens += other.tokens;
    }
}

/// Token counter using tiktoken-rs.
///
/// Uses cl100k_base tokenizer which is compatible with Claude models.
/// The tokenizer is lazily initialized and cached for reuse.
pub struct TokenCounter {
    bpe: &'static CoreBPE,
}

// Global tokenizer instance - lazily initialized once.
static TOKENIZER: OnceLock<CoreBPE> = OnceLock::new();

impl TokenCounter {
    /// Create a new token counter.
    ///
    /// This initializes the cl100k_base tokenizer if not already done.
    /// The tokenizer is cached globally for reuse.
    pub fn new() -> Result<Self, TokenCounterError> {
        let bpe = TOKENIZER.get_or_init(|| {
            tiktoken_rs::cl100k_base().expect("failed to load cl100k_base tokenizer")
        });
        Ok(Self { bpe })
    }

    /// Count tokens in a text string.
    ///
    /// # Example
    ///
    /// ```
    /// use coding_agent_cli::tokens::TokenCounter;
    ///
    /// let counter = TokenCounter::new().unwrap();
    /// let count = counter.count("Hello, world!");
    /// assert!(count.tokens > 0);
    /// ```
    pub fn count(&self, text: &str) -> TokenCount {
        if text.is_empty() {
            return TokenCount::zero();
        }
        let tokens = self.bpe.encode_ordinary(text);
        TokenCount::new(tokens.len())
    }

    /// Count tokens for a conversation message (user or assistant).
    ///
    /// This adds overhead tokens for message structure that Claude uses.
    /// Each message has approximately 4 tokens of overhead.
    pub fn count_message(&self, role: &str, content: &str) -> TokenCount {
        let content_tokens = self.count(content);
        let role_tokens = self.count(role);
        // Approximate overhead for message structure (role markers, separators)
        let overhead = TokenCount::new(4);
        content_tokens + role_tokens + overhead
    }

    /// Estimate tokens for a full conversation.
    ///
    /// Takes an iterator of (role, content) tuples and returns total tokens.
    pub fn count_conversation<'a, I>(&self, messages: I) -> TokenCount
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let mut total = TokenCount::zero();
        for (role, content) in messages {
            total += self.count_message(role, content);
        }
        // System prompt overhead
        total += TokenCount::new(3);
        total
    }
}

impl Default for TokenCounter {
    fn default() -> Self {
        Self::new().expect("failed to initialize token counter")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_count_empty() {
        let counter = TokenCounter::new().unwrap();
        let count = counter.count("");
        assert_eq!(count.tokens, 0);
    }

    #[test]
    fn test_token_count_simple() {
        let counter = TokenCounter::new().unwrap();
        let count = counter.count("Hello world");
        // "Hello world" should be 2-3 tokens
        assert!(count.tokens >= 2 && count.tokens <= 3);
    }

    #[test]
    fn test_token_count_code() {
        let counter = TokenCounter::new().unwrap();
        let code = r#"fn main() {
    println!("Hello, world!");
}"#;
        let count = counter.count(code);
        // Code snippet should be several tokens
        assert!(count.tokens >= 10);
    }

    #[test]
    fn test_token_count_unicode() {
        let counter = TokenCounter::new().unwrap();

        // Test emoji
        let emoji_count = counter.count("Hello 👋 World 🌍");
        assert!(emoji_count.tokens > 0);

        // Test CJK characters
        let cjk_count = counter.count("你好世界");
        assert!(cjk_count.tokens > 0);

        // Test mixed
        let mixed_count = counter.count("Hello 你好 👋");
        assert!(mixed_count.tokens > 0);
    }

    #[test]
    fn test_token_count_addition() {
        let a = TokenCount::new(10);
        let b = TokenCount::new(20);
        let c = a + b;
        assert_eq!(c.tokens, 30);
    }

    #[test]
    fn test_token_count_add_assign() {
        let mut a = TokenCount::new(10);
        a += TokenCount::new(5);
        assert_eq!(a.tokens, 15);
    }

    #[test]
    fn test_count_message() {
        let counter = TokenCounter::new().unwrap();
        let count = counter.count_message("user", "Hello");
        // Should include content + role + overhead
        assert!(count.tokens > counter.count("Hello").tokens);
    }

    #[test]
    fn test_count_conversation() {
        let counter = TokenCounter::new().unwrap();
        let messages = vec![
            ("user", "Hello"),
            ("assistant", "Hi there! How can I help?"),
        ];
        let count = counter.count_conversation(messages);
        // Should be sum of messages plus system overhead
        assert!(count.tokens > 0);
    }

    #[test]
    fn test_counter_is_reusable() {
        let counter = TokenCounter::new().unwrap();
        let count1 = counter.count("test");
        let count2 = counter.count("test");
        assert_eq!(count1.tokens, count2.tokens);
    }

    #[test]
    fn test_counter_default() {
        let counter = TokenCounter::default();
        let count = counter.count("test");
        assert!(count.tokens > 0);
    }
}
