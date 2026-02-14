// Quick test to see actual context bar output
use coding_agent_cli::ui::context_bar::ContextBar;

fn main() {
    let mut bar = ContextBar::new(200_000);

    println!("=== Test 1: Empty bar (0 tokens) ===");
    bar.set_tokens(0);
    println!("Tokens: {} / {}", bar.current_tokens(), bar.max_tokens());
    println!("Percent: {}%", bar.percent());
    println!("Rendered: {}", bar.render());
    println!();

    println!("=== Test 2: Low usage (87 tokens, like in screenshot) ===");
    bar.set_tokens(87);
    println!("Tokens: {} / {}", bar.current_tokens(), bar.max_tokens());
    println!("Percent: {}%", bar.percent());
    println!("Rendered: {}", bar.render());
    println!();

    println!("=== Test 3: Half full (100k tokens) ===");
    bar.set_tokens(100_000);
    println!("Tokens: {} / {}", bar.current_tokens(), bar.max_tokens());
    println!("Percent: {}%", bar.percent());
    println!("Rendered: {}", bar.render());
    println!();

    println!("=== Test 4: Full (200k tokens) ===");
    bar.set_tokens(200_000);
    println!("Tokens: {} / {}", bar.current_tokens(), bar.max_tokens());
    println!("Percent: {}%", bar.percent());
    println!("Rendered: {}", bar.render());
}
