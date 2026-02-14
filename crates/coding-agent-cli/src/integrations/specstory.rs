//! SpecStory integration for conversation persistence
//!
//! This module provides functionality to save and load conversation sessions
//! in a SpecStory-compatible markdown format.
//!
//! ## File Format
//!
//! Sessions are saved as markdown files with the following structure:
//!
//! ```markdown
//! ---
//! title: Session Title
//! created: 2024-01-15T10:30:00Z
//! updated: 2024-01-15T11:45:00Z
//! model: claude-3-opus
//! version: 1
//! ---
//!
//! # Session Title
//!
//! ## User
//!
//! User message content here...
//!
//! ## Agent
//!
//! Agent response content here...
//!
//! ## User
//!
//! Another user message...
//! ```
//!
//! Files are named using the format: `YYYY-MM-DD_HH-MM-SS_<slug>.md`
//! where `<slug>` is derived from the first user message or a default name.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// A message in a conversation session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Message {
    /// The role of the message sender
    pub role: MessageRole,
    /// The content of the message
    pub content: String,
    /// Optional timestamp for when the message was created
    pub timestamp: Option<String>,
}

/// The role of a message sender
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    /// A message from the user
    User,
    /// A message from the AI agent
    Agent,
    /// A system message (e.g., tool results)
    System,
}

impl MessageRole {
    /// Get the display name for this role
    pub fn as_str(&self) -> &'static str {
        match self {
            MessageRole::User => "User",
            MessageRole::Agent => "Agent",
            MessageRole::System => "System",
        }
    }
}

impl std::fmt::Display for MessageRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Metadata for a session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionMetadata {
    /// Title of the session (derived from first message or set manually)
    pub title: String,
    /// When the session was created (ISO 8601 format)
    pub created: String,
    /// When the session was last updated (ISO 8601 format)
    pub updated: String,
    /// The AI model used
    pub model: String,
    /// Format version number
    pub version: u32,
}

impl Default for SessionMetadata {
    fn default() -> Self {
        let now = chrono_now();
        Self {
            title: "New Session".to_string(),
            created: now.clone(),
            updated: now,
            model: "claude-3-opus".to_string(),
            version: 1,
        }
    }
}

/// A conversation session
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Session {
    /// Session metadata
    pub metadata: SessionMetadata,
    /// Messages in the session
    pub messages: Vec<Message>,
    /// Path to the session file (if loaded from disk)
    #[serde(skip)]
    pub file_path: Option<PathBuf>,
}

impl Session {
    /// Create a new empty session
    pub fn new() -> Self {
        Self {
            metadata: SessionMetadata::default(),
            messages: Vec::new(),
            file_path: None,
        }
    }

    /// Create a new session with a specific model
    pub fn with_model(model: &str) -> Self {
        let mut session = Self::new();
        session.metadata.model = model.to_string();
        session
    }

    /// Add a message to the session
    pub fn add_message(&mut self, role: MessageRole, content: &str) {
        let timestamp = chrono_now();
        self.messages.push(Message {
            role,
            content: content.to_string(),
            timestamp: Some(timestamp.clone()),
        });
        self.metadata.updated = timestamp;

        // Update title if this is the first user message
        if self.messages.len() == 1 && matches!(role, MessageRole::User) {
            self.metadata.title = derive_title(content);
        }
    }

    /// Add a user message
    pub fn add_user_message(&mut self, content: &str) {
        self.add_message(MessageRole::User, content);
    }

    /// Add an agent message
    pub fn add_agent_message(&mut self, content: &str) {
        self.add_message(MessageRole::Agent, content);
    }

    /// Add a system message
    pub fn add_system_message(&mut self, content: &str) {
        self.add_message(MessageRole::System, content);
    }

    /// Get the number of messages in the session
    pub fn message_count(&self) -> usize {
        self.messages.len()
    }

    /// Check if the session is empty
    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }

    /// Serialize the session to markdown format
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        // Write YAML frontmatter
        md.push_str("---\n");
        md.push_str(&format!(
            "title: \"{}\"\n",
            escape_yaml_string(&self.metadata.title)
        ));
        md.push_str(&format!("created: {}\n", self.metadata.created));
        md.push_str(&format!("updated: {}\n", self.metadata.updated));
        md.push_str(&format!("model: {}\n", self.metadata.model));
        md.push_str(&format!("version: {}\n", self.metadata.version));
        md.push_str("---\n\n");

        // Write title as H1
        md.push_str(&format!("# {}\n\n", self.metadata.title));

        // Write messages
        for msg in &self.messages {
            md.push_str(&format!("## {}\n\n", msg.role));
            md.push_str(&msg.content);
            if !msg.content.ends_with('\n') {
                md.push('\n');
            }
            md.push('\n');
        }

        md
    }

    /// Parse a session from markdown format
    pub fn from_markdown(content: &str) -> Result<Self, SpecStoryError> {
        // Split frontmatter and body
        let (metadata, body) = parse_frontmatter(content)?;

        // Parse messages from body
        let messages = parse_messages(body)?;

        Ok(Self {
            metadata,
            messages,
            file_path: None,
        })
    }

    /// Generate a filename for this session
    pub fn generate_filename(&self) -> String {
        let slug = slugify(&self.metadata.title);
        // Extract date and time from created timestamp
        // Format: 2024-01-15T10:30:00Z -> 2024-01-15_10-30-00
        let datetime = self
            .metadata
            .created
            .replace('T', "_")
            .replace(':', "-")
            .replace('Z', "");
        // Truncate at the seconds
        let datetime = datetime.split('.').next().unwrap_or(&datetime);
        format!("{}_{}.md", datetime, slug)
    }
}

impl Default for Session {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors that can occur during SpecStory operations
#[derive(Debug)]
pub enum SpecStoryError {
    /// Failed to read a file
    ReadError(std::io::Error),
    /// Failed to write a file
    WriteError(std::io::Error),
    /// Failed to parse the markdown format
    ParseError(String),
    /// Invalid path
    InvalidPath(String),
    /// Directory error
    DirectoryError(std::io::Error),
}

impl std::fmt::Display for SpecStoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecStoryError::ReadError(e) => write!(f, "Failed to read session file: {}", e),
            SpecStoryError::WriteError(e) => write!(f, "Failed to write session file: {}", e),
            SpecStoryError::ParseError(msg) => write!(f, "Failed to parse session: {}", msg),
            SpecStoryError::InvalidPath(msg) => write!(f, "Invalid path: {}", msg),
            SpecStoryError::DirectoryError(e) => write!(f, "Directory error: {}", e),
        }
    }
}

impl std::error::Error for SpecStoryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            SpecStoryError::ReadError(e) => Some(e),
            SpecStoryError::WriteError(e) => Some(e),
            SpecStoryError::DirectoryError(e) => Some(e),
            _ => None,
        }
    }
}

/// Manager for session files
#[derive(Clone)]
pub struct SessionManager {
    /// Base directory for session files
    base_dir: PathBuf,
}

impl SessionManager {
    /// Create a new session manager with the given base directory
    pub fn new(base_dir: PathBuf) -> Self {
        Self { base_dir }
    }

    /// Create a session manager using the default path relative to the current directory
    pub fn default_for_cwd() -> Result<Self, SpecStoryError> {
        let cwd = std::env::current_dir().map_err(SpecStoryError::DirectoryError)?;
        Ok(Self::new(cwd.join(".specstory").join("history")))
    }

    /// Get the base directory
    pub fn base_dir(&self) -> &Path {
        &self.base_dir
    }

    /// Ensure the base directory exists
    pub fn ensure_dir(&self) -> Result<(), SpecStoryError> {
        fs::create_dir_all(&self.base_dir).map_err(SpecStoryError::DirectoryError)
    }

    /// Save a session to disk
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::integrations::specstory::{SessionManager, Session, MessageRole};
    /// use std::path::PathBuf;
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SessionManager::new(PathBuf::from(".specstory/history"));
    ///
    /// // Create a new session
    /// let mut session = Session::with_model("claude-3-opus");
    /// session.add_user_message("How do I implement a binary search tree?");
    /// session.add_agent_message("Here's a binary search tree implementation...");
    ///
    /// // Save to disk
    /// let file_path = manager.save(&mut session)?;
    /// println!("Session saved to: {}", file_path.display());
    /// # Ok(())
    /// # }
    /// ```
    pub fn save(&self, session: &mut Session) -> Result<PathBuf, SpecStoryError> {
        self.ensure_dir()?;

        // Use existing file path or generate a new one
        let file_path = session
            .file_path
            .clone()
            .unwrap_or_else(|| self.base_dir.join(session.generate_filename()));

        // Update the session's file path
        session.file_path = Some(file_path.clone());

        // Write the file
        let content = session.to_markdown();
        fs::write(&file_path, content).map_err(SpecStoryError::WriteError)?;

        Ok(file_path)
    }

    /// Load a session from a file
    pub fn load(&self, filename: &str) -> Result<Session, SpecStoryError> {
        let file_path = self.base_dir.join(filename);
        self.load_from_path(&file_path)
    }

    /// Load a session from a specific path
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// use coding_agent_cli::integrations::specstory::SessionManager;
    /// use std::path::{Path, PathBuf};
    ///
    /// # fn example() -> Result<(), Box<dyn std::error::Error>> {
    /// let manager = SessionManager::new(PathBuf::from(".specstory/history"));
    ///
    /// // Load an existing session
    /// let session = manager.load_from_path(
    ///     Path::new(".specstory/history/2024-01-15_10-30-00_binary-search.md")
    /// )?;
    ///
    /// println!("Loaded session: {}", session.metadata.title);
    /// println!("Messages: {}", session.message_count());
    /// # Ok(())
    /// # }
    /// ```
    pub fn load_from_path(&self, path: &Path) -> Result<Session, SpecStoryError> {
        let content = fs::read_to_string(path).map_err(SpecStoryError::ReadError)?;
        let mut session = Session::from_markdown(&content)?;
        session.file_path = Some(path.to_path_buf());
        Ok(session)
    }

    /// List all session files, sorted by modification time (most recent first)
    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>, SpecStoryError> {
        if !self.base_dir.exists() {
            return Ok(Vec::new());
        }

        let mut sessions = Vec::new();

        let entries = fs::read_dir(&self.base_dir).map_err(SpecStoryError::DirectoryError)?;

        for entry in entries {
            let entry = entry.map_err(SpecStoryError::DirectoryError)?;
            let path = entry.path();

            if path.extension().and_then(|e| e.to_str()) == Some("md") {
                // Try to load session metadata
                if let Ok(session) = self.load_from_path(&path) {
                    let modified = entry
                        .metadata()
                        .ok()
                        .and_then(|m| m.modified().ok())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);

                    sessions.push(SessionInfo {
                        filename: path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .unwrap_or("")
                            .to_string(),
                        title: session.metadata.title,
                        created: session.metadata.created,
                        updated: session.metadata.updated,
                        message_count: session.messages.len(),
                        modified,
                    });
                }
            }
        }

        // Sort by modification time, most recent first
        sessions.sort_by(|a, b| b.modified.cmp(&a.modified));

        Ok(sessions)
    }

    /// Get the most recent session, if any
    pub fn get_latest(&self) -> Result<Option<Session>, SpecStoryError> {
        let sessions = self.list_sessions()?;
        if let Some(info) = sessions.first() {
            let session = self.load(&info.filename)?;
            Ok(Some(session))
        } else {
            Ok(None)
        }
    }

    /// Delete a session file
    pub fn delete(&self, filename: &str) -> Result<(), SpecStoryError> {
        let path = self.base_dir.join(filename);
        fs::remove_file(path).map_err(SpecStoryError::WriteError)
    }
}

/// Information about a saved session (for listing)
#[derive(Debug, Clone)]
pub struct SessionInfo {
    /// The filename
    pub filename: String,
    /// Session title
    pub title: String,
    /// When the session was created
    pub created: String,
    /// When the session was last updated
    pub updated: String,
    /// Number of messages in the session
    pub message_count: usize,
    /// File modification time (for sorting)
    modified: std::time::SystemTime,
}

impl SessionInfo {
    /// Get a human-readable "time ago" string
    pub fn time_ago(&self) -> String {
        let now = std::time::SystemTime::now();
        let duration = now.duration_since(self.modified).unwrap_or_default();
        let secs = duration.as_secs();

        if secs < 60 {
            "just now".to_string()
        } else if secs < 3600 {
            let mins = secs / 60;
            if mins == 1 {
                "1 minute ago".to_string()
            } else {
                format!("{} minutes ago", mins)
            }
        } else if secs < 86400 {
            let hours = secs / 3600;
            if hours == 1 {
                "1 hour ago".to_string()
            } else {
                format!("{} hours ago", hours)
            }
        } else {
            let days = secs / 86400;
            if days == 1 {
                "1 day ago".to_string()
            } else {
                format!("{} days ago", days)
            }
        }
    }
}

// --- Helper functions ---

/// Get current timestamp in ISO 8601 format
fn chrono_now() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();

    // Convert to date/time components (simplified, no timezone handling)
    let days_since_epoch = secs / 86400;
    let time_of_day = secs % 86400;

    let hours = time_of_day / 3600;
    let minutes = (time_of_day % 3600) / 60;
    let seconds = time_of_day % 60;

    // Calculate year, month, day from days since epoch
    // This is a simplified algorithm
    let (year, month, day) = days_to_ymd(days_since_epoch as i64);

    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        year, month, day, hours, minutes, seconds
    )
}

/// Convert days since Unix epoch to year/month/day
fn days_to_ymd(days: i64) -> (i32, u32, u32) {
    // Algorithm from http://howardhinnant.github.io/date_algorithms.html
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u32;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };

    (y as i32, m, d)
}

/// Derive a title from the first message content
fn derive_title(content: &str) -> String {
    // Take first line or first 50 chars, whichever is shorter
    let first_line = content.lines().next().unwrap_or(content);
    let truncated = if first_line.len() > 50 {
        format!("{}...", &first_line[..47])
    } else {
        first_line.to_string()
    };

    // Clean up the title
    truncated
        .trim()
        .trim_start_matches(|c: char| c.is_whitespace() || c == '#')
        .to_string()
}

/// Convert a string to a URL-safe slug
fn slugify(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect::<String>()
        .split('-')
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("-")
        .chars()
        .take(50)
        .collect()
}

/// Escape a string for YAML
fn escape_yaml_string(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// Parse YAML frontmatter from markdown content
fn parse_frontmatter(content: &str) -> Result<(SessionMetadata, &str), SpecStoryError> {
    let content = content.trim_start();

    if !content.starts_with("---") {
        return Err(SpecStoryError::ParseError(
            "Missing frontmatter delimiter".to_string(),
        ));
    }

    // Find end of frontmatter
    let after_first = &content[3..];
    let end_pos = after_first.find("\n---").ok_or_else(|| {
        SpecStoryError::ParseError("Missing closing frontmatter delimiter".to_string())
    })?;

    let frontmatter = &after_first[..end_pos].trim();
    let body = &after_first[end_pos + 4..];

    // Parse frontmatter fields
    let mut title = "Untitled Session".to_string();
    let mut created = chrono_now();
    let mut updated = created.clone();
    let mut model = "claude-3-opus".to_string();
    let mut version = 1u32;

    for line in frontmatter.lines() {
        let line = line.trim();
        if let Some((key, value)) = line.split_once(':') {
            let key = key.trim();
            let value = value.trim().trim_matches('"');
            match key {
                "title" => title = value.to_string(),
                "created" => created = value.to_string(),
                "updated" => updated = value.to_string(),
                "model" => model = value.to_string(),
                "version" => version = value.parse().unwrap_or(1),
                _ => {}
            }
        }
    }

    Ok((
        SessionMetadata {
            title,
            created,
            updated,
            model,
            version,
        },
        body,
    ))
}

/// Parse messages from the markdown body
fn parse_messages(body: &str) -> Result<Vec<Message>, SpecStoryError> {
    let mut messages = Vec::new();
    let mut current_role: Option<MessageRole> = None;
    let mut current_content = String::new();

    for line in body.lines() {
        // Check for role header (## User, ## Agent, ## System)
        if line.starts_with("## ") {
            // Save previous message if any
            if let Some(role) = current_role.take() {
                let content = current_content.trim().to_string();
                if !content.is_empty() {
                    messages.push(Message {
                        role,
                        content,
                        timestamp: None,
                    });
                }
                current_content.clear();
            }

            // Parse new role
            let role_str = line[3..].trim();
            current_role = match role_str {
                "User" => Some(MessageRole::User),
                "Agent" => Some(MessageRole::Agent),
                "System" => Some(MessageRole::System),
                _ => None,
            };
        } else if current_role.is_some() {
            // Skip H1 title line
            if line.starts_with("# ") && messages.is_empty() && current_content.is_empty() {
                continue;
            }
            // Accumulate content
            if !current_content.is_empty() || !line.is_empty() {
                if !current_content.is_empty() {
                    current_content.push('\n');
                }
                current_content.push_str(line);
            }
        }
    }

    // Save last message
    if let Some(role) = current_role {
        let content = current_content.trim().to_string();
        if !content.is_empty() {
            messages.push(Message {
                role,
                content,
                timestamp: None,
            });
        }
    }

    Ok(messages)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn test_session_new() {
        let session = Session::new();
        assert!(session.messages.is_empty());
        assert!(!session.metadata.title.is_empty());
        assert!(!session.metadata.created.is_empty());
    }

    #[test]
    fn test_session_add_message() {
        let mut session = Session::new();
        session.add_user_message("Hello, world!");
        session.add_agent_message("Hi there!");

        assert_eq!(session.message_count(), 2);
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[0].content, "Hello, world!");
        assert_eq!(session.messages[1].role, MessageRole::Agent);
        assert_eq!(session.messages[1].content, "Hi there!");
    }

    #[test]
    fn test_session_title_from_first_message() {
        let mut session = Session::new();
        session.add_user_message("What is the meaning of life?");

        assert_eq!(session.metadata.title, "What is the meaning of life?");
    }

    #[test]
    fn test_session_title_truncation() {
        let mut session = Session::new();
        session.add_user_message("This is a very long message that should be truncated because it exceeds the maximum title length");

        assert!(session.metadata.title.len() <= 50);
        assert!(session.metadata.title.ends_with("..."));
    }

    #[test]
    fn test_session_to_markdown() {
        let mut session = Session::new();
        session.metadata.created = "2024-01-15T10:30:00Z".to_string();
        session.metadata.model = "claude-3-opus".to_string();
        // Note: add_message will set the title from the first user message
        session.add_message(MessageRole::User, "Test Session Question");
        session.add_message(MessageRole::Agent, "Hi there!");

        let md = session.to_markdown();

        assert!(md.contains("---"));
        assert!(md.contains("title: \"Test Session Question\""));
        assert!(md.contains("model: claude-3-opus"));
        assert!(md.contains("## User"));
        assert!(md.contains("Test Session Question"));
        assert!(md.contains("## Agent"));
        assert!(md.contains("Hi there!"));
    }

    #[test]
    fn test_session_from_markdown() {
        let md = r#"---
title: "Test Session"
created: 2024-01-15T10:30:00Z
updated: 2024-01-15T10:31:00Z
model: claude-3-opus
version: 1
---

# Test Session

## User

Hello, how are you?

## Agent

I'm doing well, thank you!

## User

Great to hear!
"#;

        let session = Session::from_markdown(md).expect("Should parse markdown");

        assert_eq!(session.metadata.title, "Test Session");
        assert_eq!(session.metadata.model, "claude-3-opus");
        assert_eq!(session.messages.len(), 3);
        assert_eq!(session.messages[0].role, MessageRole::User);
        assert_eq!(session.messages[0].content, "Hello, how are you?");
        assert_eq!(session.messages[1].role, MessageRole::Agent);
        assert_eq!(session.messages[1].content, "I'm doing well, thank you!");
        assert_eq!(session.messages[2].role, MessageRole::User);
        assert_eq!(session.messages[2].content, "Great to hear!");
    }

    #[test]
    fn test_session_roundtrip() {
        let mut original = Session::new();
        original.metadata.title = "Roundtrip Test".to_string();
        original.metadata.model = "test-model".to_string();
        original.add_user_message("First message");
        original.add_agent_message("Response with\nmultiple lines");
        original.add_user_message("Another message");

        let md = original.to_markdown();
        let parsed = Session::from_markdown(&md).expect("Should parse roundtrip");

        assert_eq!(parsed.metadata.title, original.metadata.title);
        assert_eq!(parsed.metadata.model, original.metadata.model);
        assert_eq!(parsed.messages.len(), original.messages.len());

        for (orig, parsed) in original.messages.iter().zip(parsed.messages.iter()) {
            assert_eq!(orig.role, parsed.role);
            assert_eq!(orig.content, parsed.content);
        }
    }

    #[test]
    fn test_session_filename_generation() {
        let mut session = Session::new();
        session.metadata.title = "Test: A Complex Title!".to_string();
        session.metadata.created = "2024-01-15T10:30:00Z".to_string();

        let filename = session.generate_filename();

        assert!(filename.starts_with("2024-01-15_10-30-00_"));
        assert!(filename.ends_with(".md"));
        assert!(filename.contains("test-a-complex-title"));
    }

    #[test]
    fn test_session_manager_save_and_load() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let manager = SessionManager::new(temp_dir.path().join("history"));

        let mut session = Session::new();
        session.add_user_message("Test message");

        // Save
        let path = manager.save(&mut session).expect("Should save");
        assert!(path.exists());

        // Load
        let filename = path.file_name().unwrap().to_str().unwrap();
        let loaded = manager.load(filename).expect("Should load");

        assert_eq!(loaded.metadata.title, session.metadata.title);
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Test message");
    }

    #[test]
    fn test_session_manager_list_sessions() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let manager = SessionManager::new(temp_dir.path().join("history"));

        // Create a few sessions
        let mut session1 = Session::new();
        session1.add_user_message("First session");
        manager.save(&mut session1).expect("Should save");

        let mut session2 = Session::new();
        session2.add_user_message("Second session");
        manager.save(&mut session2).expect("Should save");

        // List sessions
        let sessions = manager.list_sessions().expect("Should list");
        assert_eq!(sessions.len(), 2);
    }

    #[test]
    fn test_session_manager_get_latest() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let manager = SessionManager::new(temp_dir.path().join("history"));

        // No sessions yet
        let latest = manager.get_latest().expect("Should not error");
        assert!(latest.is_none());

        // Create a session
        let mut session = Session::new();
        session.add_user_message("Latest test");
        manager.save(&mut session).expect("Should save");

        // Now should find it
        let latest = manager.get_latest().expect("Should not error");
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().messages[0].content, "Latest test");
    }

    #[test]
    fn test_session_manager_delete() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let manager = SessionManager::new(temp_dir.path().join("history"));

        let mut session = Session::new();
        session.add_user_message("To be deleted");
        let path = manager.save(&mut session).expect("Should save");
        let filename = path.file_name().unwrap().to_str().unwrap().to_string();

        assert!(path.exists());
        manager.delete(&filename).expect("Should delete");
        assert!(!path.exists());
    }

    #[test]
    fn test_parse_frontmatter_missing() {
        let content = "# No frontmatter here\n\nJust content.";
        let result = parse_frontmatter(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_session_load_corrupted() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let history_dir = temp_dir.path().join("history");
        fs::create_dir_all(&history_dir).expect("Failed to create dir");

        // Write corrupted file
        let corrupted_path = history_dir.join("corrupted.md");
        fs::write(&corrupted_path, "This is not valid session markdown").expect("Failed to write");

        let manager = SessionManager::new(history_dir);
        let result = manager.load("corrupted.md");

        assert!(result.is_err());
    }

    #[test]
    fn test_slugify() {
        assert_eq!(slugify("Hello World"), "hello-world");
        assert_eq!(slugify("Test: Complex! Title?"), "test-complex-title");
        assert_eq!(slugify("   Spaces   "), "spaces");
        assert_eq!(slugify("a-b-c"), "a-b-c");
    }

    #[test]
    fn test_derive_title() {
        assert_eq!(derive_title("Short title"), "Short title");
        assert_eq!(derive_title("First line\nSecond line"), "First line");
        assert!(
            derive_title("A very long title that exceeds fifty characters and should be truncated")
                .len()
                <= 50
        );
    }

    #[test]
    fn test_time_ago() {
        let mut info = SessionInfo {
            filename: "test.md".to_string(),
            title: "Test".to_string(),
            created: "2024-01-01T00:00:00Z".to_string(),
            updated: "2024-01-01T00:00:00Z".to_string(),
            message_count: 0,
            modified: std::time::SystemTime::now(),
        };

        // Just now
        assert_eq!(info.time_ago(), "just now");

        // 5 minutes ago
        info.modified = std::time::SystemTime::now() - std::time::Duration::from_secs(300);
        assert_eq!(info.time_ago(), "5 minutes ago");

        // 1 hour ago
        info.modified = std::time::SystemTime::now() - std::time::Duration::from_secs(3600);
        assert_eq!(info.time_ago(), "1 hour ago");

        // 2 hours ago
        info.modified = std::time::SystemTime::now() - std::time::Duration::from_secs(7200);
        assert_eq!(info.time_ago(), "2 hours ago");

        // 1 day ago
        info.modified = std::time::SystemTime::now() - std::time::Duration::from_secs(86400);
        assert_eq!(info.time_ago(), "1 day ago");
    }

    #[test]
    fn test_message_role_display() {
        assert_eq!(MessageRole::User.to_string(), "User");
        assert_eq!(MessageRole::Agent.to_string(), "Agent");
        assert_eq!(MessageRole::System.to_string(), "System");
    }

    #[test]
    fn test_session_with_model() {
        let session = Session::with_model("gpt-4");
        assert_eq!(session.metadata.model, "gpt-4");
    }

    #[test]
    fn test_session_is_empty() {
        let mut session = Session::new();
        assert!(session.is_empty());

        session.add_user_message("Hello");
        assert!(!session.is_empty());
    }

    #[test]
    fn test_session_system_message() {
        let mut session = Session::new();
        session.add_system_message("Tool result: success");

        assert_eq!(session.messages[0].role, MessageRole::System);
        assert_eq!(session.messages[0].content, "Tool result: success");
    }

    #[test]
    fn test_multiline_message_roundtrip() {
        let mut session = Session::new();
        let multiline = "Line 1\nLine 2\n\nLine 4 after blank";
        session.add_user_message(multiline);

        let md = session.to_markdown();
        let parsed = Session::from_markdown(&md).expect("Should parse");

        assert_eq!(parsed.messages[0].content, multiline);
    }

    #[test]
    fn test_code_block_in_message() {
        let mut session = Session::new();
        let code_msg = "Here is some code:\n\n```rust\nfn main() {\n    println!(\"Hello\");\n}\n```\n\nThat's it!";
        session.add_agent_message(code_msg);

        let md = session.to_markdown();
        let parsed = Session::from_markdown(&md).expect("Should parse");

        assert_eq!(parsed.messages[0].content, code_msg);
    }
}
