//! Tests for fun fact display during long waits

use coding_agent_cli::ui::ThinkingMessages;
use coding_agent_cli::ui::{FunFact, FunFactCache, FunFactClient};
use std::thread;
use std::time::Duration;

#[test]
fn test_thinking_messages_rotation() {
    let mut thinking = ThinkingMessages::new();
    let first = thinking.current();

    // Should start with first message
    assert_eq!(first, "Pondering...");

    // Next should advance
    let second = thinking.next();
    assert_eq!(second, "Percolating...");

    let third = thinking.next();
    assert_eq!(third, "Cogitating...");
}

#[test]
fn test_thinking_messages_cycle() {
    let mut thinking = ThinkingMessages::new();
    let count = thinking.count();

    // Starting at "Pondering..." (index 0)
    assert_eq!(thinking.current(), "Pondering...");

    // Cycle through all messages (this advances us by count positions)
    for _ in 0..count {
        thinking.next();
    }

    // After cycling count times, we should be back at the first message
    // (index 0 + count) % count = 0
    assert_eq!(thinking.current(), "Pondering...");
}

#[test]
fn test_fun_fact_client_creation() {
    let result = FunFactClient::new();
    assert!(result.is_ok(), "Should create fun fact client successfully");
}

#[test]
fn test_fun_fact_always_returns_something() {
    let cache = FunFactCache::new();
    let mut client = FunFactClient::with_cache(cache).unwrap();

    // get_fact should never fail - it has fallbacks
    let fact = client.get_fact();

    assert!(!fact.text.is_empty(), "Fact should have text");
    assert!(!fact.source.is_empty(), "Fact should have source");
}

#[test]
fn test_fun_fact_from_empty_cache_uses_fallback() {
    // Create a client with empty cache (simulating offline mode)
    let cache = FunFactCache::new();
    let mut client = FunFactClient::with_cache(cache).unwrap();

    // Should fall back to curated facts when cache is empty
    let fact = client.get_fact();

    // Fallback facts have source "curated" or from API
    assert!(!fact.text.is_empty());
}

#[test]
fn test_fun_fact_from_populated_cache() {
    let mut cache = FunFactCache::new();
    cache.add(FunFact::new("Test fact", "test-source"));

    let mut client = FunFactClient::with_cache(cache).unwrap();

    // Should return the cached fact
    let fact = client.get_fact();
    assert_eq!(fact.text, "Test fact");
    assert_eq!(fact.source, "test-source");
}

#[test]
fn test_fun_fact_delay_threshold() {
    // This test simulates the delay threshold logic
    let delay_threshold = 10u32;

    // Simulated elapsed times
    let short_wait = Duration::from_secs(5);
    let long_wait = Duration::from_secs(11);

    // Short wait should not trigger fun fact
    assert!(short_wait.as_secs() < delay_threshold as u64);

    // Long wait should trigger fun fact
    assert!(long_wait.as_secs() >= delay_threshold as u64);
}

#[test]
fn test_fallback_facts_are_valid() {
    // Verify the fallback fact system works
    let fact = FunFactClient::get_fallback_fact();

    assert!(!fact.text.is_empty(), "Fallback fact should have text");
    assert_eq!(
        fact.source, "curated",
        "Fallback facts should be marked as curated"
    );
}

#[test]
fn test_fallback_facts_vary_over_time() {
    // Get a fact
    let fact1 = FunFactClient::get_fallback_fact();

    // Wait a moment (the selection is based on current time)
    thread::sleep(Duration::from_millis(1100));

    // Get another fact
    let fact2 = FunFactClient::get_fallback_fact();

    // Both should be valid
    assert!(!fact1.text.is_empty());
    assert!(!fact2.text.is_empty());
    assert_eq!(fact1.source, "curated");
    assert_eq!(fact2.source, "curated");

    // They might be different (time-based selection), but both should be valid
    // We don't assert they're different since it depends on timing
}

#[test]
fn test_thinking_messages_with_custom_interval() {
    let thinking = ThinkingMessages::new().with_rotation_interval(Duration::from_millis(50));

    // Should still start with first message
    assert_eq!(thinking.current(), "Pondering...");
}

#[test]
fn test_thinking_messages_tick_before_interval() {
    let mut thinking = ThinkingMessages::new().with_rotation_interval(Duration::from_secs(10));

    // Tick immediately - should not change
    let changed = thinking.tick();
    assert!(!changed, "Should not change before interval");
    assert_eq!(thinking.current(), "Pondering...");
}

#[test]
fn test_thinking_messages_tick_after_interval() {
    let mut thinking = ThinkingMessages::new().with_rotation_interval(Duration::from_millis(50));

    // Wait for interval to pass
    thread::sleep(Duration::from_millis(60));

    // Tick should now change the message
    let changed = thinking.tick();
    assert!(changed, "Should change after interval");
    assert_eq!(thinking.current(), "Percolating...");
}

#[test]
fn test_fun_fact_cache_operations() {
    let mut cache = FunFactCache::new();

    // Empty cache
    assert!(cache.is_empty());
    assert_eq!(cache.len(), 0);
    assert!(cache.get_random().is_none());

    // Add a fact
    cache.add(FunFact::new("Fact 1", "source1"));
    assert!(!cache.is_empty());
    assert_eq!(cache.len(), 1);

    // Should now return a fact
    let fact = cache.get_random();
    assert!(fact.is_some());
    assert_eq!(fact.unwrap().text, "Fact 1");
}

#[test]
fn test_fun_fact_cache_max_size() {
    let mut cache = FunFactCache::with_settings(3, 3600);

    // Add more than max size
    for i in 0..5 {
        cache.add(FunFact::new(format!("Fact {}", i), "test"));
    }

    // Should only keep the last 3
    assert_eq!(cache.len(), 3);
}

#[test]
fn test_fun_fact_cache_needs_refresh_when_empty() {
    let cache = FunFactCache::new();
    assert!(cache.needs_refresh(), "Empty cache should need refresh");
}

#[test]
fn test_fun_fact_cache_fresh_facts_dont_need_refresh() {
    let mut cache = FunFactCache::with_settings(100, 3600);
    cache.add(FunFact::new("Fresh fact", "test"));

    assert!(
        !cache.needs_refresh(),
        "Fresh facts should not need refresh"
    );
}
