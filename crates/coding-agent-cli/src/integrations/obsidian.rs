//! Obsidian vault integration for note management
//!
//! This module provides functionality to search, create, and update notes
//! in an Obsidian vault.

use chrono::{DateTime, Local};
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

/// Type of note template to generate
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoteType {
    /// Meeting notes with attendees, agenda, and action items
    Meeting,
    /// Concept explanation with definition, examples, and links
    Concept,
    /// Reference documentation with structured information
    Reference,
    /// General purpose note
    General,
}

impl NoteType {
    /// Parse a note type from a string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "meeting" => Some(NoteType::Meeting),
            "concept" => Some(NoteType::Concept),
            "reference" => Some(NoteType::Reference),
            "general" => Some(NoteType::General),
            _ => None,
        }
    }

    /// Get the display name of this note type
    pub fn display_name(&self) -> &'static str {
        match self {
            NoteType::Meeting => "Meeting",
            NoteType::Concept => "Concept",
            NoteType::Reference => "Reference",
            NoteType::General => "General",
        }
    }
}

/// Metadata for a note
#[derive(Debug, Clone, PartialEq)]
pub struct NoteMetadata {
    /// Tags for categorization
    pub tags: Vec<String>,
    /// Date created
    pub created: DateTime<Local>,
    /// Related notes (backlinks)
    pub related: Vec<String>,
    /// Note type
    pub note_type: NoteType,
}

impl Default for NoteMetadata {
    fn default() -> Self {
        Self {
            tags: Vec::new(),
            created: Local::now(),
            related: Vec::new(),
            note_type: NoteType::General,
        }
    }
}

impl NoteMetadata {
    /// Create new metadata with the given type
    pub fn with_type(note_type: NoteType) -> Self {
        Self {
            note_type,
            ..Default::default()
        }
    }

    /// Add a tag
    pub fn add_tag(&mut self, tag: impl Into<String>) {
        self.tags.push(tag.into());
    }

    /// Add a related note
    pub fn add_related(&mut self, note: impl Into<String>) {
        self.related.push(note.into());
    }

    /// Format metadata as YAML frontmatter
    pub fn to_frontmatter(&self) -> String {
        let mut output = String::from("---\n");

        // Date
        output.push_str(&format!(
            "created: {}\n",
            self.created.format("%Y-%m-%d %H:%M")
        ));

        // Note type
        output.push_str(&format!("type: {}\n", self.note_type.display_name()));

        // Tags
        if !self.tags.is_empty() {
            output.push_str("tags:\n");
            for tag in &self.tags {
                output.push_str(&format!("  - {}\n", tag));
            }
        }

        // Related notes
        if !self.related.is_empty() {
            output.push_str("related:\n");
            for note in &self.related {
                output.push_str(&format!("  - \"[[{}]]\"\n", note));
            }
        }

        output.push_str("---\n\n");
        output
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

    /// Generate a note template based on type and topic
    pub fn generate_template(&self, topic: &str, note_type: NoteType) -> String {
        // Infer tags from topic
        let mut metadata = NoteMetadata::with_type(note_type);
        infer_tags_from_topic(topic, &mut metadata);

        let mut content = metadata.to_frontmatter();

        match note_type {
            NoteType::Meeting => {
                content.push_str(&format!("# {}\n\n", topic));
                content.push_str("## Attendees\n\n");
                content.push_str("- \n\n");
                content.push_str("## Agenda\n\n");
                content.push_str("1. \n\n");
                content.push_str("## Discussion\n\n");
                content.push_str("### Topic 1\n\n");
                content.push_str("\n\n");
                content.push_str("## Action Items\n\n");
                content.push_str("- [ ] \n\n");
                content.push_str("## Next Steps\n\n");
                content.push_str("\n");
            }
            NoteType::Concept => {
                content.push_str(&format!("# {}\n\n", topic));
                content.push_str("## Overview\n\n");
                content.push_str("Brief explanation of what this concept is.\n\n");
                content.push_str("## Key Points\n\n");
                content.push_str("- Point 1\n");
                content.push_str("- Point 2\n");
                content.push_str("- Point 3\n\n");
                content.push_str("## Examples\n\n");
                content.push_str("```\n// Example code or usage\n```\n\n");
                content.push_str("## Related Concepts\n\n");
                content.push_str("- \n\n");
                content.push_str("## References\n\n");
                content.push_str("- \n");
            }
            NoteType::Reference => {
                content.push_str(&format!("# {}\n\n", topic));
                content.push_str("> Quick reference for [topic]\n\n");
                content.push_str("## Quick Reference\n\n");
                content.push_str("| Item | Description |\n");
                content.push_str("|------|-------------|\n");
                content.push_str("|      |             |\n\n");
                content.push_str("## Details\n\n");
                content.push_str("### Section 1\n\n");
                content.push_str("\n\n");
                content.push_str("## Common Patterns\n\n");
                content.push_str("```\n// Pattern example\n```\n\n");
                content.push_str("## See Also\n\n");
                content.push_str("- \n");
            }
            NoteType::General => {
                content.push_str(&format!("# {}\n\n", topic));
                content.push_str("## Notes\n\n");
                content.push_str("\n");
            }
        }

        content
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

/// Infer tags from a topic string
fn infer_tags_from_topic(topic: &str, metadata: &mut NoteMetadata) {
    let topic_lower = topic.to_lowercase();

    // Programming languages
    let languages = [
        "rust",
        "python",
        "javascript",
        "typescript",
        "java",
        "c++",
        "c#",
        "go",
        "ruby",
    ];
    for lang in &languages {
        if topic_lower.contains(lang) {
            metadata.add_tag(lang.to_string());
            metadata.add_tag("programming");
        }
    }

    // General categories
    if topic_lower.contains("error") || topic_lower.contains("bug") {
        metadata.add_tag("debugging");
    }
    if topic_lower.contains("test") {
        metadata.add_tag("testing");
    }
    if topic_lower.contains("api") {
        metadata.add_tag("api");
    }
    if topic_lower.contains("design") || topic_lower.contains("architecture") {
        metadata.add_tag("design");
    }
    if topic_lower.contains("meeting") {
        metadata.add_tag("meeting");
    }
}

/// Generate a diff preview showing changes between old and new content
pub fn generate_diff(old_content: &str, new_content: &str) -> String {
    let mut output = String::new();

    let old_lines: Vec<&str> = old_content.lines().collect();
    let new_lines: Vec<&str> = new_content.lines().collect();

    // Simple line-by-line diff
    let max_lines = old_lines.len().max(new_lines.len());
    let mut added = 0;
    let mut removed = 0;
    let mut unchanged = 0;

    for i in 0..max_lines {
        let old_line = old_lines.get(i);
        let new_line = new_lines.get(i);

        match (old_line, new_line) {
            (Some(old), Some(new)) if old == new => {
                // Line unchanged
                output.push_str(&format!("  {}\n", old));
                unchanged += 1;
            }
            (Some(old), Some(new)) => {
                // Line modified
                output.push_str(&format!("- {}\n", old));
                output.push_str(&format!("+ {}\n", new));
                removed += 1;
                added += 1;
            }
            (Some(old), None) => {
                // Line removed
                output.push_str(&format!("- {}\n", old));
                removed += 1;
            }
            (None, Some(new)) => {
                // Line added
                output.push_str(&format!("+ {}\n", new));
                added += 1;
            }
            (None, None) => {
                // Should never happen
                break;
            }
        }
    }

    // Add summary
    output.push_str(&format!(
        "\n{} line(s) unchanged, {} insertion(s), {} deletion(s)\n",
        unchanged, added, removed
    ));

    output
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

    #[test]
    fn test_generate_diff_identical() {
        let old = "Line 1\nLine 2\nLine 3";
        let new = "Line 1\nLine 2\nLine 3";
        let diff = super::generate_diff(old, new);

        // Should show all lines as unchanged
        assert!(diff.contains("  Line 1"));
        assert!(diff.contains("  Line 2"));
        assert!(diff.contains("  Line 3"));
        assert!(diff.contains("3 line(s) unchanged"));
        assert!(diff.contains("0 insertion(s)"));
        assert!(diff.contains("0 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_added_lines() {
        let old = "Line 1\nLine 2";
        let new = "Line 1\nLine 2\nLine 3\nLine 4";
        let diff = super::generate_diff(old, new);

        // Should show additions
        assert!(diff.contains("+ Line 3"));
        assert!(diff.contains("+ Line 4"));
        assert!(diff.contains("2 line(s) unchanged"));
        assert!(diff.contains("2 insertion(s)"));
        assert!(diff.contains("0 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_removed_lines() {
        let old = "Line 1\nLine 2\nLine 3\nLine 4";
        let new = "Line 1\nLine 2";
        let diff = super::generate_diff(old, new);

        // Should show deletions
        assert!(diff.contains("- Line 3"));
        assert!(diff.contains("- Line 4"));
        assert!(diff.contains("2 line(s) unchanged"));
        assert!(diff.contains("0 insertion(s)"));
        assert!(diff.contains("2 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_modified_lines() {
        let old = "Line 1\nLine 2 old\nLine 3";
        let new = "Line 1\nLine 2 new\nLine 3";
        let diff = super::generate_diff(old, new);

        // Should show modification as remove + add
        assert!(diff.contains("- Line 2 old"));
        assert!(diff.contains("+ Line 2 new"));
        assert!(diff.contains("2 line(s) unchanged"));
        assert!(diff.contains("1 insertion(s)"));
        assert!(diff.contains("1 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_empty() {
        let old = "";
        let new = "";
        let diff = super::generate_diff(old, new);

        // Should show no changes
        assert!(diff.contains("0 line(s) unchanged"));
        assert!(diff.contains("0 insertion(s)"));
        assert!(diff.contains("0 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_empty_to_content() {
        let old = "";
        let new = "New line 1\nNew line 2";
        let diff = super::generate_diff(old, new);

        // Should show all additions
        assert!(diff.contains("+ New line 1"));
        assert!(diff.contains("+ New line 2"));
        assert!(diff.contains("0 line(s) unchanged"));
        assert!(diff.contains("2 insertion(s)"));
        assert!(diff.contains("0 deletion(s)"));
    }

    #[test]
    fn test_generate_diff_content_to_empty() {
        let old = "Old line 1\nOld line 2";
        let new = "";
        let diff = super::generate_diff(old, new);

        // Should show all deletions
        assert!(diff.contains("- Old line 1"));
        assert!(diff.contains("- Old line 2"));
        assert!(diff.contains("0 line(s) unchanged"));
        assert!(diff.contains("0 insertion(s)"));
        assert!(diff.contains("2 deletion(s)"));
    }

    #[test]
    fn test_note_type_from_str() {
        assert_eq!(NoteType::from_str("meeting"), Some(NoteType::Meeting));
        assert_eq!(NoteType::from_str("MEETING"), Some(NoteType::Meeting));
        assert_eq!(NoteType::from_str("concept"), Some(NoteType::Concept));
        assert_eq!(NoteType::from_str("reference"), Some(NoteType::Reference));
        assert_eq!(NoteType::from_str("general"), Some(NoteType::General));
        assert_eq!(NoteType::from_str("invalid"), None);
    }

    #[test]
    fn test_note_type_display_name() {
        assert_eq!(NoteType::Meeting.display_name(), "Meeting");
        assert_eq!(NoteType::Concept.display_name(), "Concept");
        assert_eq!(NoteType::Reference.display_name(), "Reference");
        assert_eq!(NoteType::General.display_name(), "General");
    }

    #[test]
    fn test_note_metadata_default() {
        let metadata = NoteMetadata::default();
        assert_eq!(metadata.note_type, NoteType::General);
        assert!(metadata.tags.is_empty());
        assert!(metadata.related.is_empty());
    }

    #[test]
    fn test_note_metadata_with_type() {
        let metadata = NoteMetadata::with_type(NoteType::Meeting);
        assert_eq!(metadata.note_type, NoteType::Meeting);
    }

    #[test]
    fn test_note_metadata_add_tag() {
        let mut metadata = NoteMetadata::default();
        metadata.add_tag("rust");
        metadata.add_tag("programming");
        assert_eq!(metadata.tags, vec!["rust", "programming"]);
    }

    #[test]
    fn test_note_metadata_add_related() {
        let mut metadata = NoteMetadata::default();
        metadata.add_related("Related Note 1");
        metadata.add_related("Related Note 2");
        assert_eq!(metadata.related, vec!["Related Note 1", "Related Note 2"]);
    }

    #[test]
    fn test_note_metadata_to_frontmatter() {
        let mut metadata = NoteMetadata::with_type(NoteType::Concept);
        metadata.add_tag("rust");
        metadata.add_tag("programming");
        metadata.add_related("Error Handling");

        let frontmatter = metadata.to_frontmatter();

        assert!(frontmatter.starts_with("---\n"));
        assert!(frontmatter.contains("created:"));
        assert!(frontmatter.contains("type: Concept"));
        assert!(frontmatter.contains("tags:"));
        assert!(frontmatter.contains("  - rust"));
        assert!(frontmatter.contains("  - programming"));
        assert!(frontmatter.contains("related:"));
        assert!(frontmatter.contains("  - \"[[Error Handling]]\""));
        assert!(frontmatter.ends_with("---\n\n"));
    }

    #[test]
    fn test_infer_tags_from_topic_programming() {
        let mut metadata = NoteMetadata::default();
        super::infer_tags_from_topic("rust error handling", &mut metadata);

        assert!(metadata.tags.contains(&"rust".to_string()));
        assert!(metadata.tags.contains(&"programming".to_string()));
        assert!(metadata.tags.contains(&"debugging".to_string()));
    }

    #[test]
    fn test_infer_tags_from_topic_meeting() {
        let mut metadata = NoteMetadata::default();
        super::infer_tags_from_topic("weekly team meeting", &mut metadata);

        assert!(metadata.tags.contains(&"meeting".to_string()));
    }

    #[test]
    fn test_infer_tags_from_topic_multiple() {
        let mut metadata = NoteMetadata::default();
        super::infer_tags_from_topic("python api testing", &mut metadata);

        assert!(metadata.tags.contains(&"python".to_string()));
        assert!(metadata.tags.contains(&"programming".to_string()));
        assert!(metadata.tags.contains(&"api".to_string()));
        assert!(metadata.tags.contains(&"testing".to_string()));
    }

    #[test]
    fn test_generate_template_meeting() {
        let (_temp, vault) = create_test_vault();
        let template = vault.generate_template("Weekly Team Meeting", NoteType::Meeting);

        assert!(template.contains("---")); // Frontmatter
        assert!(template.contains("type: Meeting"));
        assert!(template.contains("# Weekly Team Meeting"));
        assert!(template.contains("## Attendees"));
        assert!(template.contains("## Agenda"));
        assert!(template.contains("## Discussion"));
        assert!(template.contains("## Action Items"));
        assert!(template.contains("## Next Steps"));
    }

    #[test]
    fn test_generate_template_concept() {
        let (_temp, vault) = create_test_vault();
        let template = vault.generate_template("Rust Error Handling", NoteType::Concept);

        assert!(template.contains("---")); // Frontmatter
        assert!(template.contains("type: Concept"));
        assert!(template.contains("# Rust Error Handling"));
        assert!(template.contains("## Overview"));
        assert!(template.contains("## Key Points"));
        assert!(template.contains("## Examples"));
        assert!(template.contains("## Related Concepts"));
        assert!(template.contains("## References"));
    }

    #[test]
    fn test_generate_template_reference() {
        let (_temp, vault) = create_test_vault();
        let template = vault.generate_template("Rust Quick Reference", NoteType::Reference);

        assert!(template.contains("---")); // Frontmatter
        assert!(template.contains("type: Reference"));
        assert!(template.contains("# Rust Quick Reference"));
        assert!(template.contains("## Quick Reference"));
        assert!(template.contains("## Details"));
        assert!(template.contains("## Common Patterns"));
        assert!(template.contains("## See Also"));
    }

    #[test]
    fn test_generate_template_general() {
        let (_temp, vault) = create_test_vault();
        let template = vault.generate_template("General Topic", NoteType::General);

        assert!(template.contains("---")); // Frontmatter
        assert!(template.contains("type: General"));
        assert!(template.contains("# General Topic"));
        assert!(template.contains("## Notes"));
    }

    #[test]
    fn test_generate_template_includes_inferred_tags() {
        let (_temp, vault) = create_test_vault();
        let template = vault.generate_template("Python API Testing", NoteType::Concept);

        // Should infer tags from topic
        assert!(template.contains("tags:"));
        assert!(template.contains("- python"));
        assert!(template.contains("- programming"));
        assert!(template.contains("- api"));
        assert!(template.contains("- testing"));
    }
}
