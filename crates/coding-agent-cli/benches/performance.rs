//! Performance benchmarks for coding-agent-cli.
//!
//! Measures performance of critical operations:
//! - Token counting (should be <10ms for 10k tokens)
//! - Context bar rendering (should be <5ms)
//! - Session save/load (should be <100ms/<200ms for large sessions)
//!
//! Run with: cargo bench --package coding-agent-cli

use coding_agent_cli::{
    ContextBar, Session, SessionManager, TokenCounter,
};
use criterion::{black_box, criterion_group, criterion_main, Criterion};
use tempfile::TempDir;

// ============================================================================
// Token Counting Benchmarks
// ============================================================================

fn bench_token_count_empty(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    c.bench_function("token_count_empty", |b| {
        b.iter(|| counter.count(black_box("")))
    });
}

fn bench_token_count_small(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    let text = "Hello, world! This is a small test.";
    c.bench_function("token_count_small", |b| {
        b.iter(|| counter.count(black_box(text)))
    });
}

fn bench_token_count_medium(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    // ~1000 tokens worth of text
    let text = "The quick brown fox jumps over the lazy dog. ".repeat(50);
    c.bench_function("token_count_medium_1k", |b| {
        b.iter(|| counter.count(black_box(&text)))
    });
}

fn bench_token_count_large(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    // ~10k tokens worth of text (target: <10ms)
    let text = "The quick brown fox jumps over the lazy dog. ".repeat(500);
    c.bench_function("token_count_large_10k", |b| {
        b.iter(|| counter.count(black_box(&text)))
    });
}

fn bench_token_count_code(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    let code = r#"
fn factorial(n: u64) -> u64 {
    if n == 0 {
        1
    } else {
        n * factorial(n - 1)
    }
}

fn main() {
    println!("Factorial of 5: {}", factorial(5));
}
"#.repeat(50); // Repeat to get a meaningful size

    c.bench_function("token_count_code", |b| {
        b.iter(|| counter.count(black_box(&code)))
    });
}

fn bench_token_count_unicode(c: &mut Criterion) {
    let counter = TokenCounter::new().unwrap();
    let text = "Hello 👋 World 🌍 你好世界 ".repeat(100);
    c.bench_function("token_count_unicode", |b| {
        b.iter(|| counter.count(black_box(&text)))
    });
}

// ============================================================================
// Context Bar Rendering Benchmarks
// ============================================================================

fn bench_context_bar_render_0_percent(c: &mut Criterion) {
    let bar = ContextBar::new(200_000);
    c.bench_function("context_bar_render_0pct", |b| {
        b.iter(|| black_box(bar.render()))
    });
}

fn bench_context_bar_render_50_percent(c: &mut Criterion) {
    let mut bar = ContextBar::new(200_000);
    bar.set_tokens(100_000);
    c.bench_function("context_bar_render_50pct", |b| {
        b.iter(|| black_box(bar.render()))
    });
}

fn bench_context_bar_render_100_percent(c: &mut Criterion) {
    let mut bar = ContextBar::new(200_000);
    bar.set_tokens(200_000);
    c.bench_function("context_bar_render_100pct", |b| {
        b.iter(|| black_box(bar.render()))
    });
}

fn bench_context_bar_update(c: &mut Criterion) {
    let mut bar = ContextBar::new(200_000);
    c.bench_function("context_bar_update", |b| {
        b.iter(|| {
            bar.add_tokens(black_box(100));
            black_box(bar.render())
        })
    });
}

// ============================================================================
// Session Save/Load Benchmarks
// ============================================================================

fn create_test_session(size: &str) -> Session {
    let num_messages = match size {
        "small" => 5,
        "medium" => 50,
        "large" => 200,
        _ => 10,
    };

    let mut session = Session::new();
    for i in 0..num_messages {
        session.add_user_message(&format!("User message {}", i));
        session.add_agent_message(&format!("Assistant response {} with some more text to make it realistic. This is a longer response that might include code snippets or explanations.", i));
    }
    session
}

fn bench_session_save_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("small");

    c.bench_function("session_save_small", |b| {
        b.iter(|| {
            manager.save(black_box(&mut session)).unwrap();
        })
    });
}

fn bench_session_save_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("medium");

    c.bench_function("session_save_medium", |b| {
        b.iter(|| {
            manager.save(black_box(&mut session)).unwrap();
        })
    });
}

fn bench_session_save_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("large");

    c.bench_function("session_save_large", |b| {
        b.iter(|| {
            manager.save(black_box(&mut session)).unwrap();
        })
    });
}

fn bench_session_load_small(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("small");
    let path = manager.save(&mut session).unwrap();

    c.bench_function("session_load_small", |b| {
        b.iter(|| {
            manager.load_from_path(black_box(&path)).unwrap();
        })
    });
}

fn bench_session_load_medium(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("medium");
    let path = manager.save(&mut session).unwrap();

    c.bench_function("session_load_medium", |b| {
        b.iter(|| {
            manager.load_from_path(black_box(&path)).unwrap();
        })
    });
}

fn bench_session_load_large(c: &mut Criterion) {
    let temp_dir = TempDir::new().unwrap();
    let manager = SessionManager::new(temp_dir.path().to_path_buf());
    let mut session = create_test_session("large");
    let path = manager.save(&mut session).unwrap();

    c.bench_function("session_load_large", |b| {
        b.iter(|| {
            manager.load_from_path(black_box(&path)).unwrap();
        })
    });
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group!(
    token_counting,
    bench_token_count_empty,
    bench_token_count_small,
    bench_token_count_medium,
    bench_token_count_large,
    bench_token_count_code,
    bench_token_count_unicode
);

criterion_group!(
    context_bar,
    bench_context_bar_render_0_percent,
    bench_context_bar_render_50_percent,
    bench_context_bar_render_100_percent,
    bench_context_bar_update
);

criterion_group!(
    session_persistence,
    bench_session_save_small,
    bench_session_save_medium,
    bench_session_save_large,
    bench_session_load_small,
    bench_session_load_medium,
    bench_session_load_large
);

criterion_main!(token_counting, context_bar, session_persistence);
