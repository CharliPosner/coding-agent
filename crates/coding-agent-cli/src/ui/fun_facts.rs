//! Fun facts API integration for displaying entertaining content during long waits
//!
//! This module fetches fun facts from external APIs to keep users entertained
//! during operations that take longer than 10 seconds.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// Maximum time to wait for an API response
const API_TIMEOUT: Duration = Duration::from_secs(3);

/// Default cache size (number of facts to keep)
const DEFAULT_CACHE_SIZE: usize = 100;

/// Default refresh interval (1 hour)
const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(3600);

/// Curated fallback facts for offline use
const FALLBACK_FACTS: &[(&str, &str)] = &[
    // Programming facts
    ("The first computer bug was an actual bug—a moth found trapped in Harvard's Mark II computer in 1947.", "curated"),
    ("The average programmer spends 10-20% of their time actually writing code. The rest is spent reading code, debugging, and researching.", "curated"),
    ("The first computer programmer was Ada Lovelace, who wrote algorithms for Charles Babbage's Analytical Engine in the 1840s.", "curated"),
    ("Linux was originally created by Linus Torvalds as a hobby project while he was a student at the University of Helsinki in 1991.", "curated"),
    ("The term 'debugging' predates computers and was used in engineering to describe fixing mechanical problems.", "curated"),

    // Computing history
    ("The first 1GB hard drive, released in 1980, weighed over 500 pounds and cost $40,000.", "curated"),
    ("The first electronic computer, ENIAC, weighed 30 tons and took up 1,800 square feet of space.", "curated"),
    ("The '@' symbol in email addresses was chosen by Ray Tomlinson in 1971 simply because it wasn't used in names.", "curated"),
    ("The first computer mouse was made of wood and was invented by Douglas Engelbart in 1964.", "curated"),
    ("The original name for Windows was 'Interface Manager' but was changed before the first version shipped.", "curated"),

    // Programming languages
    ("Python is named after Monty Python's Flying Circus, not the snake.", "curated"),
    ("The first version of JavaScript was written in just 10 days by Brendan Eich in 1995.", "curated"),
    ("C++ was originally called 'C with Classes' before being renamed.", "curated"),
    ("Ruby was named after the birthstone of one of the creator's colleagues.", "curated"),
    ("The Go programming language was created by Google engineers who were frustrated waiting for C++ programs to compile.", "curated"),

    // Fun tech facts
    ("The first computer virus was created in 1983 and was called 'Elk Cloner'. It infected Apple II computers.", "curated"),
    ("The QWERTY keyboard layout was designed to slow down typing to prevent mechanical typewriters from jamming.", "curated"),
    ("The first YouTube video was uploaded on April 23, 2005, and was titled 'Me at the zoo'.", "curated"),
    ("Alaska is the only state that can be typed on one row of a traditional QWERTY keyboard.", "curated"),
    ("The first domain name ever registered was symbolics.com on March 15, 1985.", "curated"),

    // Computer science concepts
    ("The term 'bit' is short for 'binary digit' and was coined by statistician John Tukey in 1947.", "curated"),
    ("The word 'robot' comes from the Czech word 'robota', meaning forced labor or drudgery.", "curated"),
    ("The first algorithm was created by Al-Khwarizmi, a Persian mathematician from the 9th century. The word 'algorithm' comes from his name.", "curated"),
    ("ASCII was developed from telegraph code and stands for American Standard Code for Information Interchange.", "curated"),
    ("The binary number system used in computing was invented by Gottfried Wilhelm Leibniz in 1679.", "curated"),
];

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

/// Cache entry with timestamp for expiry tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CachedFact {
    /// The cached fun fact
    fact: FunFact,
    /// Timestamp when this fact was cached
    cached_at: SystemTime,
}

/// Cache for storing fun facts locally
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunFactCache {
    /// List of cached facts
    facts: Vec<CachedFact>,
    /// Maximum number of facts to store
    max_size: usize,
    /// How long facts are considered fresh (in seconds)
    refresh_interval: u64,
}

impl FunFactCache {
    /// Create a new cache with default settings
    pub fn new() -> Self {
        Self {
            facts: Vec::new(),
            max_size: DEFAULT_CACHE_SIZE,
            refresh_interval: DEFAULT_REFRESH_INTERVAL.as_secs(),
        }
    }

    /// Create a new cache with custom settings
    pub fn with_settings(max_size: usize, refresh_interval_secs: u64) -> Self {
        Self {
            facts: Vec::new(),
            max_size,
            refresh_interval: refresh_interval_secs,
        }
    }

    /// Get the default cache file path
    pub fn default_path() -> Result<PathBuf, String> {
        let cache_dir = dirs::cache_dir().ok_or("Could not determine cache directory")?;
        Ok(cache_dir.join("coding-agent").join("fun_facts.json"))
    }

    /// Load cache from the default path
    pub fn load() -> Result<Self, String> {
        let path = Self::default_path()?;
        Self::load_from(&path)
    }

    /// Load cache from a specific path
    pub fn load_from(path: &PathBuf) -> Result<Self, String> {
        if !path.exists() {
            return Ok(Self::new());
        }

        let contents =
            fs::read_to_string(path).map_err(|e| format!("Failed to read cache file: {}", e))?;

        serde_json::from_str(&contents).map_err(|e| format!("Failed to parse cache file: {}", e))
    }

    /// Save cache to the default path
    pub fn save(&self) -> Result<(), String> {
        let path = Self::default_path()?;
        self.save_to(&path)
    }

    /// Save cache to a specific path
    pub fn save_to(&self, path: &PathBuf) -> Result<(), String> {
        // Ensure parent directory exists
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create cache directory: {}", e))?;
        }

        let contents = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize cache: {}", e))?;

        fs::write(path, contents).map_err(|e| format!("Failed to write cache file: {}", e))
    }

    /// Add a fact to the cache
    pub fn add(&mut self, fact: FunFact) {
        let cached = CachedFact {
            fact,
            cached_at: SystemTime::now(),
        };

        self.facts.push(cached);

        // Enforce max size by removing oldest entries
        if self.facts.len() > self.max_size {
            let excess = self.facts.len() - self.max_size;
            self.facts.drain(0..excess);
        }
    }

    /// Get a random fact from the cache
    pub fn get_random(&self) -> Option<FunFact> {
        if self.facts.is_empty() {
            return None;
        }

        // Use a simple pseudo-random selection based on current time
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        let index = (now.as_secs() as usize) % self.facts.len();
        Some(self.facts[index].fact.clone())
    }

    /// Check if the cache needs refreshing (all entries are old)
    pub fn needs_refresh(&self) -> bool {
        if self.facts.is_empty() {
            return true;
        }

        let now = SystemTime::now();
        let refresh_duration = Duration::from_secs(self.refresh_interval);

        // Check if any facts are still fresh
        self.facts.iter().all(|cached| {
            now.duration_since(cached.cached_at)
                .map(|age| age > refresh_duration)
                .unwrap_or(true)
        })
    }

    /// Get the number of facts in the cache
    pub fn len(&self) -> usize {
        self.facts.len()
    }

    /// Check if the cache is empty
    pub fn is_empty(&self) -> bool {
        self.facts.is_empty()
    }

    /// Clear all cached facts
    pub fn clear(&mut self) {
        self.facts.clear();
    }
}

impl Default for FunFactCache {
    fn default() -> Self {
        Self::new()
    }
}

/// Client for fetching fun facts from various APIs
#[derive(Debug, Clone)]
pub struct FunFactClient {
    client: reqwest::Client,
    cache: FunFactCache,
}

impl FunFactClient {
    /// Create a new fun fact client with default timeout and load cache
    pub fn new() -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        // Try to load existing cache, or create a new one if it fails
        let cache = FunFactCache::load().unwrap_or_else(|_| FunFactCache::new());

        Ok(Self { client, cache })
    }

    /// Create a new fun fact client with an existing cache
    pub fn with_cache(cache: FunFactCache) -> Result<Self, String> {
        let client = reqwest::Client::builder()
            .timeout(API_TIMEOUT)
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

        Ok(Self { client, cache })
    }

    /// Get a reference to the cache
    pub fn cache(&self) -> &FunFactCache {
        &self.cache
    }

    /// Save the current cache to disk
    pub fn save_cache(&self) -> Result<(), String> {
        self.cache.save()
    }

    /// Fetch a fun fact from the specified source
    pub async fn fetch(&self, source: FactSource) -> Result<FunFact, String> {
        match source {
            FactSource::UselessFacts => self.fetch_useless_fact().await,
            FactSource::Jokes => self.fetch_joke().await,
            FactSource::Quotes => self.fetch_quote().await,
        }
    }

    /// Fetch a random fun fact from any available source
    pub async fn fetch_random(&self) -> Result<FunFact, String> {
        // Try each source in order until one succeeds
        let sources = [
            FactSource::UselessFacts,
            FactSource::Jokes,
            FactSource::Quotes,
        ];

        let mut last_error = String::new();

        for source in sources {
            match self.fetch(source).await {
                Ok(fact) => return Ok(fact),
                Err(e) => last_error = e,
            }
        }

        Err(format!(
            "All API sources failed. Last error: {}",
            last_error
        ))
    }

    /// Get a random fact from the curated fallback list
    ///
    /// This method provides offline facts when the API is unavailable
    /// and no cached facts exist.
    pub fn get_fallback_fact() -> FunFact {
        // Use time-based pseudo-random selection
        let now = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap_or(Duration::from_secs(0));

        let index = (now.as_secs() as usize) % FALLBACK_FACTS.len();
        let (text, source) = FALLBACK_FACTS[index];
        FunFact::new(text.to_string(), source.to_string())
    }

    /// Get a fun fact, using cache if available or fetching if needed
    ///
    /// This method will:
    /// 1. Try to get a fact from the cache
    /// 2. If cache is empty or needs refresh, fetch from API
    /// 3. Add newly fetched facts to the cache
    /// 4. If all else fails, fall back to curated list
    /// 5. Always returns a fact (never fails)
    pub async fn get_fact(&mut self) -> FunFact {
        // If cache is empty or needs refresh, try to fetch and cache new facts
        if self.cache.is_empty() || self.cache.needs_refresh() {
            // Try to fetch a fact and add it to cache
            match self.fetch_random().await {
                Ok(fact) => {
                    self.cache.add(fact.clone());
                    // Try to save cache (ignore errors, we still have the fact)
                    let _ = self.cache.save();
                    return fact;
                }
                Err(_) => {
                    // If fetch fails but we have cached facts, use them
                    if let Some(cached_fact) = self.cache.get_random() {
                        return cached_fact;
                    }
                    // No cache and fetch failed - use fallback
                    return Self::get_fallback_fact();
                }
            }
        }

        // Cache is fresh, use it
        self.cache
            .get_random()
            .unwrap_or_else(|| Self::get_fallback_fact())
    }

    /// Get a fun fact synchronously from cache or fallback only (no API calls)
    ///
    /// This is useful when you need a fact but can't await.
    /// It will return a cached fact if available, or a fallback fact.
    pub fn get_fact_sync(&self) -> FunFact {
        self.cache
            .get_random()
            .unwrap_or_else(|| Self::get_fallback_fact())
    }

    /// Preload cache with multiple facts from various sources
    ///
    /// This method fetches facts from all sources and adds them to the cache.
    /// Useful for warming up the cache in the background.
    pub async fn preload_cache(&mut self, count: usize) -> Result<usize, String> {
        let sources = [
            FactSource::UselessFacts,
            FactSource::Jokes,
            FactSource::Quotes,
        ];

        let mut loaded = 0;

        for _ in 0..count {
            // Rotate through sources
            let source = sources[loaded % sources.len()];

            match self.fetch(source).await {
                Ok(fact) => {
                    self.cache.add(fact);
                    loaded += 1;
                }
                Err(_) => {
                    // Continue trying other sources
                    continue;
                }
            }
        }

        if loaded > 0 {
            // Save the cache
            self.cache.save()?;
            Ok(loaded)
        } else {
            Err("Failed to load any facts".to_string())
        }
    }

    async fn fetch_useless_fact(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://uselessfacts.jsph.pl/random.json?language=en")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch useless fact: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Useless facts API returned status: {}",
                response.status()
            ));
        }

        let fact: UselessFactResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse useless fact response: {}", e))?;

        Ok(FunFact::new(fact.text, "uselessfacts.jsph.pl"))
    }

    async fn fetch_joke(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://official-joke-api.appspot.com/random_joke")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch joke: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Joke API returned status: {}", response.status()));
        }

        let joke: JokeResponse = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse joke response: {}", e))?;

        let text = format!("{}\n{}", joke.setup, joke.punchline);
        Ok(FunFact::new(text, "official-joke-api.appspot.com"))
    }

    async fn fetch_quote(&self) -> Result<FunFact, String> {
        let response = self
            .client
            .get("https://api.quotable.io/random")
            .send()
            .await
            .map_err(|e| format!("Failed to fetch quote: {}", e))?;

        if !response.status().is_success() {
            return Err(format!("Quote API returned status: {}", response.status()));
        }

        let quote: QuoteResponse = response
            .json()
            .await
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
    use std::thread;
    use tempfile::TempDir;

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

    // Cache tests
    #[test]
    fn test_cache_new() {
        let cache = FunFactCache::new();
        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
        assert_eq!(cache.max_size, DEFAULT_CACHE_SIZE);
    }

    #[test]
    fn test_cache_with_settings() {
        let cache = FunFactCache::with_settings(50, 7200);
        assert!(cache.is_empty());
        assert_eq!(cache.max_size, 50);
        assert_eq!(cache.refresh_interval, 7200);
    }

    #[test]
    fn test_cache_add() {
        let mut cache = FunFactCache::new();
        let fact = FunFact::new("Test fact", "test-source");

        cache.add(fact.clone());
        assert_eq!(cache.len(), 1);
        assert!(!cache.is_empty());
    }

    #[test]
    fn test_cache_add_multiple() {
        let mut cache = FunFactCache::new();

        for i in 0..10 {
            let fact = FunFact::new(format!("Fact {}", i), "test-source");
            cache.add(fact);
        }

        assert_eq!(cache.len(), 10);
    }

    #[test]
    fn test_cache_max_size_enforcement() {
        let mut cache = FunFactCache::with_settings(5, 3600);

        // Add more facts than max size
        for i in 0..10 {
            let fact = FunFact::new(format!("Fact {}", i), "test-source");
            cache.add(fact);
        }

        // Should only keep the last 5
        assert_eq!(cache.len(), 5);
    }

    #[test]
    fn test_cache_get_random() {
        let mut cache = FunFactCache::new();

        // Empty cache returns None
        assert!(cache.get_random().is_none());

        // Add some facts
        for i in 0..5 {
            let fact = FunFact::new(format!("Fact {}", i), "test-source");
            cache.add(fact);
        }

        // Should return a fact
        let fact = cache.get_random();
        assert!(fact.is_some());
        assert!(fact.unwrap().text.starts_with("Fact "));
    }

    #[test]
    fn test_cache_get_random_different_over_time() {
        let mut cache = FunFactCache::new();

        // Add multiple facts
        for i in 0..10 {
            let fact = FunFact::new(format!("Fact {}", i), "test-source");
            cache.add(fact);
        }

        // Get facts at different times (sleep between calls)
        let fact1 = cache.get_random().unwrap();
        thread::sleep(Duration::from_millis(10));
        let fact2 = cache.get_random().unwrap();

        // Due to time-based selection, these might be different
        // (not guaranteed, but likely with 10 facts)
        // Just verify they are both valid
        assert!(fact1.text.starts_with("Fact "));
        assert!(fact2.text.starts_with("Fact "));
    }

    #[test]
    fn test_cache_needs_refresh_empty() {
        let cache = FunFactCache::new();
        assert!(cache.needs_refresh(), "Empty cache should need refresh");
    }

    #[test]
    fn test_cache_needs_refresh_fresh() {
        let mut cache = FunFactCache::with_settings(100, 3600);
        let fact = FunFact::new("Fresh fact", "test-source");
        cache.add(fact);

        assert!(
            !cache.needs_refresh(),
            "Newly added facts should not need refresh"
        );
    }

    #[test]
    fn test_cache_needs_refresh_old() {
        let mut cache = FunFactCache::with_settings(100, 0); // 0 second refresh interval

        let fact = FunFact::new("Old fact", "test-source");
        cache.add(fact);

        // Wait a moment to make it "old"
        thread::sleep(Duration::from_millis(10));

        assert!(
            cache.needs_refresh(),
            "Facts older than refresh interval should need refresh"
        );
    }

    #[test]
    fn test_cache_clear() {
        let mut cache = FunFactCache::new();

        for i in 0..5 {
            let fact = FunFact::new(format!("Fact {}", i), "test-source");
            cache.add(fact);
        }

        assert_eq!(cache.len(), 5);

        cache.clear();

        assert!(cache.is_empty());
        assert_eq!(cache.len(), 0);
    }

    #[test]
    fn test_cache_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir.path().join("fun_facts.json");

        // Create and populate cache
        let mut cache = FunFactCache::new();
        cache.add(FunFact::new("Fact 1", "source1"));
        cache.add(FunFact::new("Fact 2", "source2"));

        // Save cache
        cache.save_to(&cache_path).expect("Should save cache");

        // Verify file exists
        assert!(cache_path.exists());

        // Load cache
        let loaded = FunFactCache::load_from(&cache_path).expect("Should load cache");

        assert_eq!(loaded.len(), 2);
        assert_eq!(loaded.max_size, cache.max_size);
        assert_eq!(loaded.refresh_interval, cache.refresh_interval);
    }

    #[test]
    fn test_cache_load_nonexistent() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir.path().join("nonexistent.json");

        // Loading nonexistent file should return empty cache
        let cache = FunFactCache::load_from(&cache_path).expect("Should create new cache");

        assert!(cache.is_empty());
    }

    #[test]
    fn test_cache_load_corrupted() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir.path().join("corrupted.json");

        // Write invalid JSON
        fs::write(&cache_path, "not valid json").expect("Should write file");

        // Loading corrupted file should fail gracefully
        let result = FunFactCache::load_from(&cache_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_cache_creates_parent_dir() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let cache_path = temp_dir
            .path()
            .join("nested")
            .join("dir")
            .join("fun_facts.json");

        let cache = FunFactCache::new();
        cache.save_to(&cache_path).expect("Should save cache");

        assert!(cache_path.exists());
    }

    #[test]
    fn test_client_with_cache() {
        let cache = FunFactCache::new();
        let client = FunFactClient::with_cache(cache);

        assert!(client.is_ok());
    }

    #[test]
    fn test_client_cache_access() {
        let mut cache = FunFactCache::new();
        cache.add(FunFact::new("Test fact", "test-source"));

        let client = FunFactClient::with_cache(cache).unwrap();

        assert_eq!(client.cache().len(), 1);
    }

    #[test]
    fn test_client_get_fact_sync_from_empty_cache() {
        let cache = FunFactCache::new();
        let client = FunFactClient::with_cache(cache).unwrap();

        // Since cache is empty, should return fallback fact
        let fact = client.get_fact_sync();

        // Should always return a fact (from cache or fallback)
        assert!(!fact.text.is_empty());
        assert!(!fact.source.is_empty());
    }

    #[test]
    fn test_client_get_fact_sync_from_populated_cache() {
        let mut cache = FunFactCache::new();
        cache.add(FunFact::new("Cached fact", "test-source"));

        let client = FunFactClient::with_cache(cache).unwrap();

        // Should return the cached fact
        let fact = client.get_fact_sync();
        assert_eq!(fact.text, "Cached fact");
    }

    #[tokio::test]
    async fn test_client_get_fact_from_empty_cache() {
        let cache = FunFactCache::new();
        let mut client = FunFactClient::with_cache(cache).unwrap();

        // Since cache is empty and API calls would be needed,
        // this should fall back to the curated list (or succeed if network is available)
        let fact = client.get_fact().await;

        // Should always return a fact (from API, cache, or fallback)
        assert!(!fact.text.is_empty());
        assert!(!fact.source.is_empty());
    }

    #[tokio::test]
    async fn test_client_get_fact_from_populated_cache() {
        let mut cache = FunFactCache::new();
        cache.add(FunFact::new("Cached fact", "test-source"));

        let mut client = FunFactClient::with_cache(cache).unwrap();

        // Should return the cached fact
        let fact = client.get_fact().await;
        assert_eq!(fact.text, "Cached fact");
    }

    #[test]
    fn test_get_fallback_fact() {
        let fact = FunFactClient::get_fallback_fact();

        // Should return a valid fact from the curated list
        assert!(!fact.text.is_empty());
        assert_eq!(fact.source, "curated");
    }

    #[test]
    fn test_fallback_fact_deterministic_at_same_time() {
        // Getting the fallback fact at roughly the same time should return the same fact
        let fact1 = FunFactClient::get_fallback_fact();
        let fact2 = FunFactClient::get_fallback_fact();

        assert_eq!(fact1.text, fact2.text);
        assert_eq!(fact1.source, fact2.source);
    }

    #[test]
    fn test_fallback_fact_varies_over_time() {
        // Get a fact
        let fact1 = FunFactClient::get_fallback_fact();

        // Wait a moment
        thread::sleep(Duration::from_millis(1100));

        // Get another fact (should likely be different due to time-based selection)
        let fact2 = FunFactClient::get_fallback_fact();

        // Both should be valid
        assert!(!fact1.text.is_empty());
        assert!(!fact2.text.is_empty());
        assert_eq!(fact1.source, "curated");
        assert_eq!(fact2.source, "curated");
    }

    #[test]
    fn test_fallback_facts_coverage() {
        // Verify we have a good number of curated facts
        assert!(
            FALLBACK_FACTS.len() >= 20,
            "Should have at least 20 curated facts"
        );

        // Verify all facts have non-empty text and source
        for (text, source) in FALLBACK_FACTS {
            assert!(!text.is_empty(), "Fact text should not be empty");
            assert_eq!(*source, "curated", "Source should be 'curated'");
        }
    }

    #[tokio::test]
    async fn test_get_fact_falls_back_when_cache_and_api_fail() {
        // Create a client with an empty cache
        // In offline mode, the API will fail
        let cache = FunFactCache::new();
        let mut client = FunFactClient::with_cache(cache).unwrap();

        // get_fact should never fail - it will use fallback if needed
        let fact = client.get_fact().await;

        assert!(!fact.text.is_empty());
        // The fact could be from cache, API (if online), or fallback
        // but it should always return something
    }

    // Note: The following tests make real network calls and may be slow or fail if APIs are down
    // In a production environment, these would use mocked HTTP responses

    #[tokio::test]
    #[ignore] // Ignored by default to avoid network calls in CI
    async fn test_fetch_useless_fact() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::UselessFacts).await;

        // This might fail if the API is down, which is expected
        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "uselessfacts.jsph.pl");
        }
    }

    #[tokio::test]
    #[ignore] // Ignored by default to avoid network calls in CI
    async fn test_fetch_joke() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::Jokes).await;

        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "official-joke-api.appspot.com");
        }
    }

    #[tokio::test]
    #[ignore] // Ignored by default to avoid network calls in CI
    async fn test_fetch_quote() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch(FactSource::Quotes).await;

        if let Ok(fact) = result {
            assert!(!fact.text.is_empty());
            assert_eq!(fact.source, "quotable.io");
        }
    }

    #[tokio::test]
    #[ignore] // Ignored by default to avoid network calls in CI
    async fn test_fetch_random() {
        let client = FunFactClient::new().unwrap();
        let result = client.fetch_random().await;

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

    // =========================================================================
    // Async context safety tests
    // =========================================================================
    // These tests verify that FunFactClient can be safely used and dropped
    // within an async context. This catches issues like the "Cannot drop a
    // runtime in a context where blocking is not allowed" panic that occurs
    // when reqwest::blocking::Client is used inside a tokio runtime.
    // =========================================================================

    #[tokio::test]
    async fn test_client_safe_to_create_in_async_context() {
        // This test verifies that creating a FunFactClient inside an async
        // context doesn't cause panics. Previously, using reqwest::blocking::Client
        // would create a nested runtime that panicked on drop.
        let client = FunFactClient::new();
        assert!(client.is_ok(), "Should create client without panic");
    }

    #[tokio::test]
    async fn test_client_safe_to_drop_in_async_context() {
        // This test verifies that dropping a FunFactClient inside an async
        // context doesn't cause panics. This was the root cause of the
        // "/clear command panic" bug.
        {
            let _client = FunFactClient::new().unwrap();
            // Client will be dropped here
        }
        // If we reach here without panic, the test passes
    }

    #[tokio::test]
    async fn test_client_safe_to_use_and_drop_in_async_context() {
        // Comprehensive test: create, use, and drop within async context
        let mut client = FunFactClient::new().unwrap();

        // Use the sync method (simulates what REPL does)
        let fact = client.get_fact_sync();
        assert!(!fact.text.is_empty());

        // Use the async method
        let fact_async = client.get_fact().await;
        assert!(!fact_async.text.is_empty());

        // Client will be dropped when function exits
    }

    #[tokio::test]
    async fn test_multiple_clients_in_async_context() {
        // Test that multiple clients can coexist and be dropped in async context
        let client1 = FunFactClient::new().unwrap();
        let client2 = FunFactClient::new().unwrap();

        let fact1 = client1.get_fact_sync();
        let fact2 = client2.get_fact_sync();

        assert!(!fact1.text.is_empty());
        assert!(!fact2.text.is_empty());

        // Both clients dropped here
    }

    #[tokio::test]
    async fn test_client_replacement_in_async_context() {
        // Simulates what happens during /clear when state is reset
        let mut client = FunFactClient::new().unwrap();
        let _ = client.get_fact_sync();

        // Replace the client (old one gets dropped)
        client = FunFactClient::new().unwrap();
        let _ = client.get_fact_sync();

        // If we reach here without panic, replacement is safe
    }
}
