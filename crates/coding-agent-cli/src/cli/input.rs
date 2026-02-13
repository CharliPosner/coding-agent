use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use std::time::Duration;

/// Result of reading user input
#[derive(Debug, Clone, PartialEq)]
pub enum InputResult {
    /// User submitted input (double-enter)
    Submitted(String),
    /// User cancelled input (Ctrl+C)
    Cancelled,
    /// User requested exit (Ctrl+D)
    Exit,
}

/// Handles multi-line input with double-enter submission
pub struct InputHandler {
    buffer: String,
    /// Tracks if the last key was Enter (for double-enter detection)
    last_was_enter: bool,
}

impl InputHandler {
    pub fn new() -> Self {
        Self {
            buffer: String::new(),
            last_was_enter: false,
        }
    }

    /// Read input from the terminal until submission, cancellation, or exit
    pub async fn read_input(&mut self) -> Result<InputResult, String> {
        self.buffer.clear();
        self.last_was_enter = false;

        loop {
            // Poll for events with a timeout
            if event::poll(Duration::from_millis(100)).map_err(|e| e.to_string())? {
                if let Event::Key(key_event) = event::read().map_err(|e| e.to_string())? {
                    match self.handle_key_event(key_event) {
                        KeyAction::Continue => continue,
                        KeyAction::Submit => {
                            let text = self.buffer.trim_end().to_string();
                            return Ok(InputResult::Submitted(text));
                        }
                        KeyAction::Cancel => {
                            self.buffer.clear();
                            return Ok(InputResult::Cancelled);
                        }
                        KeyAction::Exit => {
                            return Ok(InputResult::Exit);
                        }
                    }
                }
            }
        }
    }

    /// Process a key event and return the action to take
    fn handle_key_event(&mut self, event: KeyEvent) -> KeyAction {
        match (event.code, event.modifiers) {
            // Ctrl+C: Cancel current input
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => {
                self.last_was_enter = false;
                KeyAction::Cancel
            }

            // Ctrl+D: Exit application
            (KeyCode::Char('d'), KeyModifiers::CONTROL) => {
                self.last_was_enter = false;
                KeyAction::Exit
            }

            // Enter: Add newline or submit on double-enter
            (KeyCode::Enter, KeyModifiers::NONE) => {
                if self.last_was_enter {
                    // Double-enter: submit
                    // Remove the trailing newline from the first enter
                    if self.buffer.ends_with('\n') {
                        self.buffer.pop();
                    }
                    KeyAction::Submit
                } else {
                    // First enter: add newline
                    // In raw mode, need \r\n for proper line break
                    self.buffer.push('\n');
                    self.last_was_enter = true;
                    print!("\r\n");
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                    KeyAction::Continue
                }
            }

            // Backspace: Remove last character
            (KeyCode::Backspace, _) => {
                self.last_was_enter = false;
                if !self.buffer.is_empty() {
                    let removed = self.buffer.pop();
                    // Handle visual backspace
                    if removed == Some('\n') {
                        // Move cursor up and to end of previous line
                        // This is simplified - in a real app we'd track line lengths
                        print!("\x1b[A\x1b[999C");
                    } else {
                        // Simple backspace: move back, write space, move back
                        print!("\x08 \x08");
                    }
                    let _ = std::io::Write::flush(&mut std::io::stdout());
                }
                KeyAction::Continue
            }

            // Regular character input
            (KeyCode::Char(c), KeyModifiers::NONE | KeyModifiers::SHIFT) => {
                self.last_was_enter = false;
                self.buffer.push(c);
                print!("{}", c);
                let _ = std::io::Write::flush(&mut std::io::stdout());
                KeyAction::Continue
            }

            // Tab: Insert spaces (or tab character)
            (KeyCode::Tab, _) => {
                self.last_was_enter = false;
                self.buffer.push_str("    ");
                print!("    ");
                let _ = std::io::Write::flush(&mut std::io::stdout());
                KeyAction::Continue
            }

            // Ignore other keys
            _ => {
                self.last_was_enter = false;
                KeyAction::Continue
            }
        }
    }

    /// Get the current buffer contents (for testing)
    #[cfg(test)]
    pub fn buffer(&self) -> &str {
        &self.buffer
    }

    /// Check if last key was enter (for testing)
    #[cfg(test)]
    pub fn last_was_enter(&self) -> bool {
        self.last_was_enter
    }

    /// Simulate a key event (for testing)
    #[cfg(test)]
    pub fn simulate_key(&mut self, event: KeyEvent) -> KeyAction {
        self.handle_key_event(event)
    }
}

impl Default for InputHandler {
    fn default() -> Self {
        Self::new()
    }
}

/// Internal action resulting from a key press
#[derive(Debug, Clone, PartialEq)]
pub enum KeyAction {
    Continue,
    Submit,
    Cancel,
    Exit,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    fn key_event(code: KeyCode, modifiers: KeyModifiers) -> KeyEvent {
        KeyEvent {
            code,
            modifiers,
            kind: KeyEventKind::Press,
            state: KeyEventState::NONE,
        }
    }

    #[test]
    fn test_double_enter_detection() {
        let mut handler = InputHandler::new();

        // First enter adds newline
        let action = handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Continue);
        assert!(handler.last_was_enter());
        assert_eq!(handler.buffer(), "\n");

        // Second enter submits
        let action = handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Submit);
        // Buffer should have trailing newline removed
        assert_eq!(handler.buffer(), "");
    }

    #[test]
    fn test_single_enter_adds_newline() {
        let mut handler = InputHandler::new();

        // Type some text
        handler.simulate_key(key_event(KeyCode::Char('h'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));

        // Single enter adds newline
        let action = handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Continue);
        assert_eq!(handler.buffer(), "hi\n");
        assert!(handler.last_was_enter());

        // Typing another character resets the enter flag
        handler.simulate_key(key_event(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(!handler.last_was_enter());
        assert_eq!(handler.buffer(), "hi\nx");
    }

    #[test]
    fn test_ctrl_c_clears_input() {
        let mut handler = InputHandler::new();

        // Type some text
        handler.simulate_key(key_event(KeyCode::Char('t'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('s'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('t'), KeyModifiers::NONE));

        assert_eq!(handler.buffer(), "test");

        // Ctrl+C cancels
        let action = handler.simulate_key(key_event(KeyCode::Char('c'), KeyModifiers::CONTROL));
        assert_eq!(action, KeyAction::Cancel);
    }

    #[test]
    fn test_ctrl_d_exits() {
        let mut handler = InputHandler::new();

        let action = handler.simulate_key(key_event(KeyCode::Char('d'), KeyModifiers::CONTROL));
        assert_eq!(action, KeyAction::Exit);
    }

    #[test]
    fn test_backspace_removes_char() {
        let mut handler = InputHandler::new();

        // Type "abc"
        handler.simulate_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('b'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('c'), KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "abc");

        // Backspace removes 'c'
        handler.simulate_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "ab");

        // Backspace removes 'b'
        handler.simulate_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "a");
    }

    #[test]
    fn test_backspace_on_empty_buffer() {
        let mut handler = InputHandler::new();

        // Backspace on empty buffer should do nothing
        let action = handler.simulate_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Continue);
        assert_eq!(handler.buffer(), "");
    }

    #[test]
    fn test_backspace_across_newline() {
        let mut handler = InputHandler::new();

        // Type "hi" then enter
        handler.simulate_key(key_event(KeyCode::Char('h'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "hi\n");

        // Backspace removes the newline
        handler.simulate_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "hi");
    }

    #[test]
    fn test_unicode_input() {
        let mut handler = InputHandler::new();

        // Type unicode characters
        handler.simulate_key(key_event(KeyCode::Char('日'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('本'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('語'), KeyModifiers::NONE));

        assert_eq!(handler.buffer(), "日本語");

        // Backspace removes one character
        handler.simulate_key(key_event(KeyCode::Backspace, KeyModifiers::NONE));
        assert_eq!(handler.buffer(), "日本");
    }

    #[test]
    fn test_shift_characters() {
        let mut handler = InputHandler::new();

        // Type uppercase with shift
        handler.simulate_key(key_event(KeyCode::Char('H'), KeyModifiers::SHIFT));
        handler.simulate_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('!'), KeyModifiers::SHIFT));

        assert_eq!(handler.buffer(), "Hi!");
    }

    #[test]
    fn test_tab_inserts_spaces() {
        let mut handler = InputHandler::new();

        handler.simulate_key(key_event(KeyCode::Char('x'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Tab, KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('y'), KeyModifiers::NONE));

        assert_eq!(handler.buffer(), "x    y");
    }

    #[test]
    fn test_enter_after_text_resets_on_other_input() {
        let mut handler = InputHandler::new();

        // Type and press enter
        handler.simulate_key(key_event(KeyCode::Char('a'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert!(handler.last_was_enter());

        // Type more - should reset the enter flag
        handler.simulate_key(key_event(KeyCode::Char('b'), KeyModifiers::NONE));
        assert!(!handler.last_was_enter());

        // Now enter should add newline, not submit
        let action = handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Continue);
        assert_eq!(handler.buffer(), "a\nb\n");
    }

    #[test]
    fn test_multi_line_input() {
        let mut handler = InputHandler::new();

        // Type first line
        handler.simulate_key(key_event(KeyCode::Char('l'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('n'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('1'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

        // Type second line
        handler.simulate_key(key_event(KeyCode::Char('l'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('i'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('n'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('e'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Char('2'), KeyModifiers::NONE));
        handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));

        // Double-enter to submit (removes trailing newline from second enter)
        let action = handler.simulate_key(key_event(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(action, KeyAction::Submit);
        // The trailing newline from the first Enter in double-enter is removed on submit
        assert_eq!(handler.buffer(), "line1\nline2");
    }
}
