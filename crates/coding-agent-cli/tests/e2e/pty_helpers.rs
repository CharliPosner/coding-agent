use expectrl::Session;
use std::time::Duration;

/// Helper functions for PTY interaction testing

/// Wait for a specific pattern with timeout
pub fn expect_pattern(
    session: &mut Session,
    pattern: &str,
    timeout: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    session.set_expect_timeout(Some(timeout));
    let output = session.expect(pattern)?;
    Ok(String::from_utf8_lossy(output.as_bytes()).to_string())
}

/// Send text without newline
pub fn send_text(session: &mut Session, text: &str) -> Result<(), Box<dyn std::error::Error>> {
    session.send(text)?;
    Ok(())
}

/// Send a single key
pub fn send_key(session: &mut Session, key: char) -> Result<(), Box<dyn std::error::Error>> {
    session.send(&key.to_string())?;
    Ok(())
}

/// Send Enter key
pub fn send_enter(session: &mut Session) -> Result<(), Box<dyn std::error::Error>> {
    session.send("\r")?;
    Ok(())
}

/// Send backspace key
pub fn send_backspace(session: &mut Session) -> Result<(), Box<dyn std::error::Error>> {
    session.send("\x7f")?;
    Ok(())
}

/// Wait for prompt to appear
pub fn wait_for_prompt(
    session: &mut Session,
    timeout: Duration,
) -> Result<(), Box<dyn std::error::Error>> {
    session.set_expect_timeout(Some(timeout));
    session.expect(">")?;
    Ok(())
}

/// Capture output until prompt appears
pub fn capture_until_prompt(
    session: &mut Session,
    timeout: Duration,
) -> Result<String, Box<dyn std::error::Error>> {
    session.set_expect_timeout(Some(timeout));
    let output = session.expect(">")?;
    Ok(String::from_utf8_lossy(output.as_bytes()).to_string())
}

/// Check if a pattern exists in the captured output
pub fn contains_pattern(output: &str, pattern: &str) -> bool {
    output.contains(pattern)
}

/// Strip ANSI escape codes from output
pub fn strip_ansi(text: &str) -> String {
    // Simple ANSI escape sequence remover
    let re = regex::Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    re.replace_all(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi() {
        let colored = "\x1b[32mHello\x1b[0m World";
        let plain = strip_ansi(colored);
        assert_eq!(plain, "Hello World");
    }

    #[test]
    fn test_contains_pattern() {
        let output = "Some text with pattern inside";
        assert!(contains_pattern(output, "pattern"));
        assert!(!contains_pattern(output, "missing"));
    }
}
