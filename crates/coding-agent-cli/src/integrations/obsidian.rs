//! Obsidian vault integration for note management
//!
//! This module provides functionality to search, create, and update notes
//! in an Obsidian vault.

use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Errors that can occur during Obsidian operations
#[derive(Debug)]
pub enum ObsidianError {
    /// Vault path not configured
    VaultNotConfigured,
    /// Vault directory doesn't exist
    VaultNotFound(PathBuf),
    /// Failed to read vault contents
    ReadError(std::io::Error),
    /// Failed to write note
    WriteError(std::io::Error),
    /// Invalid note name
    InvalidNoteName(String),
}

impl std::fmt::Display for ObsidianError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ObsidianError::VaultNotConfigured => {
                write!(
                    f,
                    "Obsidian vault path not configured. Please set it in your config file."
                )
            }
            ObsidianError::VaultNotFound(path) => {
                write!(f, "Obsidian vault not found at: {}", path.display())
            }
            ObsidianError::ReadError(e) => write!(f, "Failed to read vault: {}", e),
            ObsidianError::WriteError(e) => write!(f, "Failed to write note: {}", e),
            ObsidianError::InvalidNoteName(name) => write!(f, "Invalid note name: {}", name),
        }
    }
}

impl std::error::Error for ObsidianError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            ObsidianError::ReadError(e) => Some(e),
            ObsidianError::WriteError(e) => Some(e),
            _ => None,
        }
    }
}

/// Represents an Obsidian note
#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    /// Full path to the note file
    pub path: PathBuf,
    /// Title of the note (filename without .md extension)
    pub title: String,
    /// Content of the note
    pub content: String,
}

impl Note {
    /// Create a new note with the given path and content
    pub fn new(path: PathBuf, content: String) -> Self {
        let title = path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Untitled")
            .to_string();

        Self {
            path,
            title,
            content,
        }
    }

    /// Load a note from a file
    pub fn load(path: &Path) -> Result<Self, ObsidianError> {
        let content = fs::read_to_string(path).map_err(ObsidianError::ReadError)?;
        Ok(Self::new(path.to_path_buf(), content))
    }

    /// Save the note to disk
    pub fn save(&self) -> Result<(), ObsidianError> {
        // Ensure parent directory exists
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent).map_err(ObsidianError::WriteError)?;
        }

        fs::write(&self.path, &self.content).map_err(ObsidianError::WriteError)
    }
}

/// Search result for a note query
#[derive(Debug, Clone, PartialEq)]
pub struct SearchResult {
    /// The note that matched
    pub note: Note,
    /// Relevance score (0.0 - 1.0)
    pub score: f32,
    /// Matching excerpt from the note
    pub excerpt: Option<String>,
}

/// Obsidian vault manager
pub struct ObsidianVault {
    /// Path to the vault directory
    vault_path: PathBuf,
}

impl ObsidianVault {
    /// Create a new vault manager with the given path
    pub fn new(vault_path: PathBuf) -> Result<Self, ObsidianError> {
        // Expand tilde in path
        let vault_path = expand_tilde(vault_path);

        if !vault_path.exists() {
            return Err(ObsidianError::VaultNotFound(vault_path));
        }

        Ok(Self { vault_path })
    }

    /// Search for notes related to a topic
    pub fn search(&self, query: &str) -> Result<Vec<SearchResult>, ObsidianError> {
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        // Walk through vault directory
        for entry in WalkDir::new(&self.vault_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            // Only process markdown files
            if !path.is_file() || path.extension().and_then(|s| s.to_str()) != Some("md") {
                continue;
            }

            // Load the note
            if let Ok(note) = Note::load(path) {
                // Calculate relevance score based on title and content matching
                let title_lower = note.title.to_lowercase();
                let content_lower = note.content.to_lowercase();

                let title_matches = title_lower.contains(&query_lower);
                let content_matches = content_lower.contains(&query_lower);

                if title_matches || content_matches {
                    // Calculate score (title matches are worth more)
                    let score = if title_matches { 0.8 } else { 0.5 };

                    // Extract excerpt around the match
                    let excerpt = if content_matches {
                        extract_excerpt(&note.content, &query_lower, 100)
                    } else {
                        None
                    };

                    results.push(SearchResult {
                        note,
                        score,
                        excerpt,
                    });
                }
            }
        }

        // Sort by relevance (highest score first)
        results.sort_by(|a, b| b.score.partial_cmp(&a.score).unwrap());

        Ok(results)
    }

    /// Create a new note at the given relative path within the vault
    pub fn create_note(&self, relative_path: &str, content: &str) -> Result<Note, ObsidianError> {
        // Validate the path doesn't try to escape the vault
        if relative_path.contains("..") {
            return Err(ObsidianError::InvalidNoteName(
                "Path cannot contain '..'".to_string(),
            ));
        }

        let note_path = self.vault_path.join(relative_path);

        // Ensure it has .md extension
        let note_path = if note_path.extension().and_then(|s| s.to_str()) != Some("md") {
            note_path.with_extension("md")
        } else {
            note_path
        };

        let note = Note::new(note_path, content.to_string());
        note.save()?;

        Ok(note)
    }

    /// Update an existing note
    pub fn update_note(&self, note: &Note) -> Result<(), ObsidianError> {
        note.save()
    }

    /// Get the vault path
    pub fn path(&self) -> &Path {
        &self.vault_path
    }

    /// Suggest a location for a new note based on topic
    pub fn suggest_location(&self, topic: &str) -> String {
        // Simple heuristic: suggest Programming/ for code-related topics
        let topic_lower = topic.to_lowercase();

        if topic_lower.contains("rust")
            || topic_lower.contains("python")
            || topic_lower.contains("javascript")
            || topic_lower.contains("code")
            || topic_lower.contains("programming")
        {
            format!("Programming/{}.md", sanitize_filename(topic))
        } else {
            format!("{}.md", sanitize_filename(topic))
        }
    }
}

/// Expand tilde (~) in path to home directory
fn expand_tilde(path: PathBuf) -> PathBuf {
    if let Some(path_str) = path.to_str() {
        if path_str.starts_with("~/") || path_str == "~" {
            if let Some(home) = dirs::home_dir() {
                return home.join(&path_str[2..]);
            }
        }
    }
    path
}

/// Extract an excerpt from content around a query match
fn extract_excerpt(content: &str, query: &str, max_len: usize) -> Option<String> {
    let content_lower = content.to_lowercase();
    let query_lower = query.to_lowercase();

    if let Some(pos) = content_lower.find(&query_lower) {
        // Get context before and after
        let start = pos.saturating_sub(max_len / 2);
        let end = (pos + query.len() + max_len / 2).min(content.len());

        let excerpt = &content[start..end];

        // Add ellipsis if we're not at the start/end
        let prefix = if start > 0 { "..." } else { "" };
        let suffix = if end < content.len() { "..." } else { "" };

        Some(format!("{}{}{}", prefix, excerpt.trim(), suffix))
    } else {
        None
    }
}

/// Sanitize a topic string to be a valid filename
fn sanitize_filename(topic: &str) -> String {
    topic
        .chars()
        .map(|c| match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | ' ' => c,
            _ => '-',
        })
        .collect::<String>()
        .trim()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_vault() -> (TempDir, ObsidianVault) {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let vault_path = temp_dir.path().to_path_buf();

        // Create some test notes
        fs::write(
            vault_path.join("rust-basics.md"),
            "# Rust Basics\n\nRust is a systems programming language.",
        )
        .expect("Failed to write test note");

        fs::create_dir_all(vault_path.join("Programming")).expect("Failed to create dir");
        fs::write(
            vault_path.join("Programming/error-handling.md"),
            "# Error Handling\n\nHow to handle errors in Rust using Result.",
        )
        .expect("Failed to write test note");

        let vault = ObsidianVault::new(vault_path).expect("Failed to create vault");
        (temp_dir, vault)
    }

    #[test]
    fn test_vault_creation() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let vault = ObsidianVault::new(temp_dir.path().to_path_buf());
        assert!(vault.is_ok());
    }

    #[test]
    fn test_vault_not_found() {
        let vault = ObsidianVault::new(PathBuf::from("/nonexistent/path"));
        assert!(matches!(vault, Err(ObsidianError::VaultNotFound(_))));
    }

    #[test]
    fn test_search_by_title() {
        let (_temp, vault) = create_test_vault();

        let results = vault.search("rust").expect("Search failed");
        assert!(!results.is_empty());

        // Should find the rust-basics note
        let has_rust_basics = results.iter().any(|r| r.note.title == "rust-basics");
        assert!(has_rust_basics);
    }

    #[test]
    fn test_search_by_content() {
        let (_temp, vault) = create_test_vault();

        let results = vault.search("error").expect("Search failed");
        assert!(!results.is_empty());

        // Should find the error-handling note
        let has_error_handling = results.iter().any(|r| r.note.title == "error-handling");
        assert!(has_error_handling);
    }

    #[test]
    fn test_search_no_results() {
        let (_temp, vault) = create_test_vault();

        let results = vault.search("nonexistent topic").expect("Search failed");
        assert!(results.is_empty());
    }

    #[test]
    fn test_create_note() {
        let (_temp, vault) = create_test_vault();

        let note = vault
            .create_note("test-note.md", "# Test\n\nThis is a test note.")
            .expect("Failed to create note");

        assert_eq!(note.title, "test-note");
        assert!(note.path.exists());
    }

    #[test]
    fn test_create_note_in_subdirectory() {
        let (_temp, vault) = create_test_vault();

        let note = vault
            .create_note("Notes/new-note.md", "# New Note")
            .expect("Failed to create note");

        assert_eq!(note.title, "new-note");
        assert!(note.path.exists());
        assert!(note.path.parent().unwrap().ends_with("Notes"));
    }

    #[test]
    fn test_create_note_adds_md_extension() {
        let (_temp, vault) = create_test_vault();

        let note = vault
            .create_note("test-note", "# Test")
            .expect("Failed to create note");

        assert!(note.path.extension().unwrap() == "md");
    }

    #[test]
    fn test_create_note_rejects_path_traversal() {
        let (_temp, vault) = create_test_vault();

        let result = vault.create_note("../../../etc/passwd", "malicious");
        assert!(matches!(result, Err(ObsidianError::InvalidNoteName(_))));
    }

    #[test]
    fn test_update_note() {
        let (_temp, vault) = create_test_vault();

        // Create a note
        let mut note = vault
            .create_note("update-test.md", "Original content")
            .expect("Failed to create note");

        // Update it
        note.content = "Updated content".to_string();
        vault.update_note(&note).expect("Failed to update note");

        // Load it again and verify
        let loaded = Note::load(&note.path).expect("Failed to load note");
        assert_eq!(loaded.content, "Updated content");
    }

    #[test]
    fn test_suggest_location_programming() {
        let (_temp, vault) = create_test_vault();

        let location = vault.suggest_location("rust error handling");
        assert!(location.starts_with("Programming/"));
        assert!(location.ends_with(".md"));
    }

    #[test]
    fn test_suggest_location_general() {
        let (_temp, vault) = create_test_vault();

        let location = vault.suggest_location("general topic");
        assert!(!location.starts_with("Programming/"));
        assert!(location.ends_with(".md"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(sanitize_filename("hello world"), "hello world");
        assert_eq!(sanitize_filename("hello/world"), "hello-world");
        assert_eq!(sanitize_filename("hello:world"), "hello-world");
        assert_eq!(sanitize_filename("hello*world"), "hello-world");
    }

    #[test]
    fn test_extract_excerpt() {
        let content = "This is a long piece of text that contains the word error somewhere in the middle and continues for a while afterwards.";
        let excerpt = extract_excerpt(content, "error", 40);

        assert!(excerpt.is_some());
        let excerpt = excerpt.unwrap();
        assert!(excerpt.contains("error"));
        assert!(excerpt.len() <= 100); // Should be around max_len + query
    }

    #[test]
    fn test_note_load_and_save() {
        let temp_dir = TempDir::new().expect("Failed to create temp dir");
        let note_path = temp_dir.path().join("test.md");

        // Create and save a note
        let note = Note::new(note_path.clone(), "# Test\n\nContent".to_string());
        note.save().expect("Failed to save note");

        // Load it back
        let loaded = Note::load(&note_path).expect("Failed to load note");
        assert_eq!(loaded.title, "test");
        assert_eq!(loaded.content, "# Test\n\nContent");
    }
}
