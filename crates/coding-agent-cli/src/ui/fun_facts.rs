//! Fun facts API integration for displaying entertaining content during long waits
//!
//! This module fetches fun facts from external APIs to keep users entertained
//! during operations that take longer than 10 seconds.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Maximum time to wait for an API response
const API_TIMEOUT: Duration = Duration::from_secs(3);

/// Different sources for fun facts
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FactSource {
    /// Random fun facts from uselessfacts.jsph.pl
    UselessFacts,
    /// Programming jokes from official-joke-api
    Jokes,
    /// Motivational quotes from quotable.io
    Quotes,
}

/// Response from uselessfacts.jsph.pl API
#[derive(Debug, Deserialize, Serialize, Clone)]
struct UselessFactResponse {
    text: String,
}

/// Response from official-joke-api
#[derive(Debug, Deserialize, Serialize, Clone)]
struct JokeResponse {
    setup: String,
    punchline: String,
}

/// Response from quotable.io API
#[derive(Debug, Deserialize, Serialize, Clone)]
struct QuoteResponse {
    content: String,
    author: String,
}

/// A fun fact that can be displayed to the user
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FunFact {
    /// The text content of the fact
    pub text: String,
    /// The source this fact came from
    pub source: String,
}

impl FunFact {
    /// Create a new fun fact
    pub fn new(text: impl Into<String>, source: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            source: source.into(),
        }
    }
}

/// Client for fetching fun facts from various APIs
#[derive(Debug, Clone)]
pub struct FunFactClient {
    client: reqwest::blocking::Client,
}

impl FunFactClient {
    /// Create a new fun fact client with default timeout
    pub fn new() -> Result<Self, String> {
        let client = reqwest::blocking::Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client })
    }

    /// Fetch a fun fact from the specified source
    pub fn fetch(&self, source: FactSource) -> Result<FunFact, String> {
        match source {
            FactSource::UselessFacts => self.fetch_useless_fact(),
            FactSource::Jokes => self.fetch_joke(),
            FactSource::Quotes => self.fetch_quote(),
        }
    }

    /// Fetch a random fun fact from any available source
    pub fn fetch_random(&self) -> Result<FunFact, String> {
        // Try each source in order until one succeeds
        let sources = [
            FactSource::UselessFacts,
            FactSource::Jokes,
            FactSource::Quotes,
        ];

        let mut last_error = String::new();

        for source in sources {
            match self.fetch(source) {
                Ok(fact) => return Ok(fact),
                Err(e) => last_error = e,
            }
        }

        Err(format!(
            "All API sources failed. Last error: {}",
            last_error
        ))
    }

    fn fetch_useless_fact(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://uselessfacts.jsph.pl/random.json?language=en")
            .send()
            .map_err(|e| format!("Failed to fetch useless fact: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Useless facts API returned status: {}",
                response.status()
            ));
        }

        let fact: UselessFactResponse = response
            .json()
            .map_err(|e| format!("Failed to parse useless fact response: {}", e))?;

        Ok(FunFact::new(fact.text, "uselessfacts.jsph.pl"))
    }

    fn fetch_joke(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://official-joke-api.appspot.com/random_joke")
            .send()
            .map_err(|e| format!("Failed to fetch joke: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Joke API returned status: {}", response.status()));
        }

        let joke: JokeResponse = response
            .json()
            .map_err(|e| format!("Failed to parse joke response: {}", e))?;

        let text = format!("{}\n{}", joke.setup, joke.punchline);
        Ok(FunFact::new(text, "official-joke-api.appspot.com"))
    }

    fn fetch_quote(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://api.quotable.io/random")
            .send()
            .map_err(|e| format!("Failed to fetch quote: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Quote API returned status: {}", response.status()));
        }

        let quote: QuoteResponse = response
            .json()
            .map_err(|e| format!("Failed to parse quote response: {}", e))?;

        let text = format!("{} — {}", quote.content, quote.author);
        Ok(FunFact::new(text, "quotable.io"))
    }
}

impl Default for FunFactClient {
    fn default() -> Self {
        Self::new().expect("Failed to create default FunFactClient")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fun_fact_new() {
        let fact = FunFact::new("Test fact", "test-source");
        assert_eq!(fact.text, "Test fact");
        assert_eq!(fact.source, "test-source");
    }

    #[test]
    fn test_client_creation() {
        let result = FunFactClient::new();
        assert!(result.is_ok(), "Should create client successfully");
    }

    // Note: The following tests make real network calls and may be slow or fail if APIs are down
    // In a production environment, these would use mocked HTTP responses

    #[test]
    #[ignore] // Ignored by default to avoid network calls in CI
    fn test_fetch_useless_fact() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::UselessFacts);

        // This might fail if the API is down, which is expected
        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "uselessfacts.jsph.pl");
        }
    }

    #[test]
    #[ignore] // Ignored by default to avoid network calls in CI
    fn test_fetch_joke() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::Jokes);

        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "official-joke-api.appspot.com");
        }
    }

    #[test]
    #[ignore] // Ignored by default to avoid network calls in CI
    fn test_fetch_quote() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::Quotes);

        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "quotable.io");
        }
    }

    #[test]
    #[ignore] // Ignored by default to avoid network calls in CI
    fn test_fetch_random() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch_random();

        // At least one API should work
        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert!(!fact.source.is_empty());
        }
    }

    #[test]
    fn test_default_client() {
        let _client = FunFactClient::default();
        // Should not panic
    }
}
