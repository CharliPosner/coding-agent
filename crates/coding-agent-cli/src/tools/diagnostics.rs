//! Diagnostic analysis for parsing compiler error messages.
//!
//! This module provides functionality to parse compiler error output (Rust, TypeScript, etc.)
//! into structured diagnostic information that can be used by the fix-agent for automatic
//! error recovery.
//!
//! Note: This module is fully implemented but not yet integrated into the main execution flow.
//! It will be activated when Phase 14.3 (Self-Healing Error Recovery) integration is completed.

#![allow(dead_code)]

use std::collections::HashMap;

/// A parsed diagnostic from compiler output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// The severity level of the diagnostic.
    pub severity: DiagnosticSeverity,

    /// The error code (e.g., "E0463", "TS2304").
    pub code: Option<String>,

    /// The main error message.
    pub message: String,

    /// The primary location of the error.
    pub location: Option<DiagnosticLocation>,

    /// Related locations (e.g., where something was previously defined).
    pub related_locations: Vec<DiagnosticLocation>,

    /// Suggested fixes from the compiler.
    pub suggestions: Vec<DiagnosticSuggestion>,

    /// Additional notes or help text from the compiler.
    pub notes: Vec<String>,

    /// The raw compiler output for this diagnostic.
    pub raw_output: String,
}

impl Diagnostic {
    /// Create a new diagnostic with just a message.
    pub fn new(message: impl Into<String>) -> Self {
        let message = message.into();
        Self {
            severity: DiagnosticSeverity::Error,
            code: None,
            message: message.clone(),
            location: None,
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: message,
        }
    }

    /// Check if this diagnostic has a known error code.
    pub fn has_code(&self) -> bool {
        self.code.is_some()
    }

    /// Check if this diagnostic has location information.
    pub fn has_location(&self) -> bool {
        self.location.is_some()
    }

    /// Check if this diagnostic has any suggestions.
    pub fn has_suggestions(&self) -> bool {
        !self.suggestions.is_empty()
    }

    /// Get the primary file path if available.
    pub fn file_path(&self) -> Option<&str> {
        self.location.as_ref().map(|l| l.file.as_str())
    }

    /// Get the primary line number if available.
    pub fn line(&self) -> Option<u32> {
        self.location.as_ref().map(|l| l.line)
    }

    /// Get the primary column number if available.
    pub fn column(&self) -> Option<u32> {
        self.location.as_ref().and_then(|l| l.column)
    }
}

/// Severity level of a diagnostic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticSeverity {
    /// A compile error - must be fixed.
    Error,
    /// A warning - should be addressed.
    Warning,
    /// Informational note.
    Note,
    /// Help message with suggestions.
    Help,
}

impl std::fmt::Display for DiagnosticSeverity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiagnosticSeverity::Error => write!(f, "error"),
            DiagnosticSeverity::Warning => write!(f, "warning"),
            DiagnosticSeverity::Note => write!(f, "note"),
            DiagnosticSeverity::Help => write!(f, "help"),
        }
    }
}

/// Location of a diagnostic in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLocation {
    /// The file path.
    pub file: String,

    /// The line number (1-indexed).
    pub line: u32,

    /// The column number (1-indexed), if available.
    pub column: Option<u32>,

    /// The end line, if this is a range.
    pub end_line: Option<u32>,

    /// The end column, if this is a range.
    pub end_column: Option<u32>,

    /// The source code snippet at this location.
    pub snippet: Option<String>,
}

impl DiagnosticLocation {
    /// Create a new location.
    pub fn new(file: impl Into<String>, line: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column: None,
            end_line: None,
            end_column: None,
            snippet: None,
        }
    }

    /// Create a location with a column.
    pub fn with_column(file: impl Into<String>, line: u32, column: u32) -> Self {
        Self {
            file: file.into(),
            line,
            column: Some(column),
            end_line: None,
            end_column: None,
            snippet: None,
        }
    }

    /// Set the end position for a range.
    pub fn with_end(mut self, end_line: u32, end_column: Option<u32>) -> Self {
        self.end_line = Some(end_line);
        self.end_column = end_column;
        self
    }

    /// Set the source code snippet.
    pub fn with_snippet(mut self, snippet: impl Into<String>) -> Self {
        self.snippet = Some(snippet.into());
        self
    }

    /// Format as "file:line" or "file:line:column".
    pub fn format_short(&self) -> String {
        match self.column {
            Some(col) => format!("{}:{}:{}", self.file, self.line, col),
            None => format!("{}:{}", self.file, self.line),
        }
    }
}

impl std::fmt::Display for DiagnosticLocation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_short())
    }
}

/// A suggested fix from the compiler.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSuggestion {
    /// Description of the suggestion.
    pub message: String,

    /// The location where the fix should be applied.
    pub location: Option<DiagnosticLocation>,

    /// The replacement text (if applicable).
    pub replacement: Option<String>,

    /// Whether this suggestion can be automatically applied.
    pub is_applicable: bool,
}

impl DiagnosticSuggestion {
    /// Create a new suggestion.
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
            location: None,
            replacement: None,
            is_applicable: false,
        }
    }

    /// Create an applicable suggestion with a replacement.
    pub fn with_replacement(
        message: impl Into<String>,
        location: DiagnosticLocation,
        replacement: impl Into<String>,
    ) -> Self {
        Self {
            message: message.into(),
            location: Some(location),
            replacement: Some(replacement.into()),
            is_applicable: true,
        }
    }
}

/// Result of parsing compiler output.
#[derive(Debug, Clone)]
pub struct DiagnosticReport {
    /// All parsed diagnostics.
    pub diagnostics: Vec<Diagnostic>,

    /// The compiler that generated this output.
    pub compiler: CompilerType,

    /// Summary statistics.
    pub error_count: usize,
    pub warning_count: usize,
}

impl DiagnosticReport {
    /// Create a new empty report.
    pub fn new(compiler: CompilerType) -> Self {
        Self {
            diagnostics: Vec::new(),
            compiler,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Check if there are any errors.
    pub fn has_errors(&self) -> bool {
        self.error_count > 0
    }

    /// Check if there are any warnings.
    pub fn has_warnings(&self) -> bool {
        self.warning_count > 0
    }

    /// Get all errors.
    pub fn errors(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Error)
    }

    /// Get all warnings.
    pub fn warnings(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics
            .iter()
            .filter(|d| d.severity == DiagnosticSeverity::Warning)
    }

    /// Get diagnostics grouped by file.
    pub fn by_file(&self) -> HashMap<&str, Vec<&Diagnostic>> {
        let mut grouped: HashMap<&str, Vec<&Diagnostic>> = HashMap::new();
        for diag in &self.diagnostics {
            if let Some(path) = diag.file_path() {
                grouped.entry(path).or_default().push(diag);
            }
        }
        grouped
    }
}

/// Type of compiler that generated the output.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CompilerType {
    /// Rust compiler (rustc/cargo).
    Rust,
    /// TypeScript compiler (tsc).
    TypeScript,
    /// Go compiler.
    Go,
    /// Generic/unknown compiler.
    Unknown,
}

impl std::fmt::Display for CompilerType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompilerType::Rust => write!(f, "rust"),
            CompilerType::TypeScript => write!(f, "typescript"),
            CompilerType::Go => write!(f, "go"),
            CompilerType::Unknown => write!(f, "unknown"),
        }
    }
}

/// Parse compiler output into structured diagnostics.
///
/// # Examples
///
/// ```rust,no_run
/// use coding_agent_cli::tools::parse_compiler_output;
///
/// let cargo_output = r#"
/// error[E0463]: can't find crate for `serde_json`
///  --> src/main.rs:1:5
///   |
/// 1 | use serde_json;
///   |     ^^^^^^^^^^ help: you might be missing crate `serde_json`
/// "#;
///
/// let report = parse_compiler_output(cargo_output);
///
/// println!("Compiler: {}", report.compiler);
/// println!("Errors: {}", report.error_count);
///
/// for error in report.errors() {
///     if let Some(code) = &error.code {
///         println!("Error {}: {}", code, error.message);
///     }
///     if let Some(file) = error.file_path() {
///         println!("  at {}:{}", file, error.line().unwrap_or(0));
///     }
/// }
/// ```
pub fn parse_compiler_output(output: &str) -> DiagnosticReport {
    // Try to detect the compiler type
    let compiler = detect_compiler(output);

    let mut report = DiagnosticReport::new(compiler);

    match compiler {
        CompilerType::Rust => parse_rust_output(output, &mut report),
        CompilerType::TypeScript => parse_typescript_output(output, &mut report),
        CompilerType::Go => parse_go_output(output, &mut report),
        CompilerType::Unknown => parse_generic_output(output, &mut report),
    }

    report
}

/// Detect which compiler generated the output.
fn detect_compiler(output: &str) -> CompilerType {
    // Rust: "error[E0xxx]:" or "warning[E0xxx]:" or "error: " with rustc patterns
    if output.contains("error[E") || output.contains("warning[E") {
        return CompilerType::Rust;
    }
    if output.contains("Compiling ") && output.contains("Finished ") {
        return CompilerType::Rust;
    }
    if output.contains("--> ") && output.contains(" |") {
        return CompilerType::Rust;
    }

    // TypeScript: "error TS2xxx:" pattern
    if output.contains("error TS") || output.contains("): error TS") {
        return CompilerType::TypeScript;
    }

    // Go: "filename.go:line:col:" pattern
    if output.lines().any(|line| {
        let parts: Vec<&str> = line.split(':').collect();
        parts.len() >= 3 && parts[0].ends_with(".go")
    }) {
        return CompilerType::Go;
    }

    CompilerType::Unknown
}

/// Parse Rust compiler output.
fn parse_rust_output(output: &str, report: &mut DiagnosticReport) {
    let lines: Vec<&str> = output.lines().collect();
    let mut i = 0;

    while i < lines.len() {
        let line = lines[i];

        // Match "error[E0463]: can't find crate for `serde_json`"
        // or "error: ..." without a code
        // or "warning[E0xxx]: ..."
        if let Some(diag) = parse_rust_diagnostic_line(line) {
            let mut diagnostic = diag;
            diagnostic.raw_output = line.to_string();

            // Look for location on next lines: "--> src/main.rs:10:5"
            i += 1;
            while i < lines.len() {
                let next_line = lines[i];

                if next_line.trim().starts_with("--> ") {
                    if let Some(loc) = parse_rust_location(next_line) {
                        diagnostic.location = Some(loc);
                    }
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                } else if next_line.trim().starts_with('|') {
                    // Source code snippet line
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                } else if next_line.trim().starts_with("= note:") {
                    let note = next_line.trim().trim_start_matches("= note:").trim();
                    diagnostic.notes.push(note.to_string());
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                } else if next_line.trim().starts_with("= help:") {
                    let help = next_line.trim().trim_start_matches("= help:").trim();
                    diagnostic.suggestions.push(DiagnosticSuggestion::new(help));
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                } else if next_line.trim().starts_with("help:") {
                    let help = next_line.trim().trim_start_matches("help:").trim();
                    diagnostic.suggestions.push(DiagnosticSuggestion::new(help));
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                } else if next_line.is_empty()
                    || next_line.starts_with("error")
                    || next_line.starts_with("warning")
                {
                    // End of this diagnostic
                    break;
                } else {
                    diagnostic.raw_output.push('\n');
                    diagnostic.raw_output.push_str(next_line);
                    i += 1;
                }
            }

            match diagnostic.severity {
                DiagnosticSeverity::Error => report.error_count += 1,
                DiagnosticSeverity::Warning => report.warning_count += 1,
                _ => {}
            }
            report.diagnostics.push(diagnostic);
        } else {
            i += 1;
        }
    }
}

/// Parse a single Rust diagnostic line.
fn parse_rust_diagnostic_line(line: &str) -> Option<Diagnostic> {
    let line = line.trim();

    // "error[E0463]: can't find crate for `serde_json`"
    if line.starts_with("error[E") {
        let end_bracket = line.find(']')?;
        let code = line[6..end_bracket].to_string();
        let message = line[end_bracket + 2..].trim_start_matches(':').trim();
        return Some(Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some(code),
            message: message.to_string(),
            location: None,
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        });
    }

    // "warning[E0599]: no method named `foo` found"
    if line.starts_with("warning[") {
        let end_bracket = line.find(']')?;
        let code = line[8..end_bracket].to_string();
        let message = line[end_bracket + 2..].trim_start_matches(':').trim();
        return Some(Diagnostic {
            severity: DiagnosticSeverity::Warning,
            code: Some(code),
            message: message.to_string(),
            location: None,
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        });
    }

    // "error: ..." without a code
    if line.starts_with("error:") {
        let message = line[6..].trim();
        if !message.is_empty() {
            return Some(Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: None,
                message: message.to_string(),
                location: None,
                related_locations: Vec::new(),
                suggestions: Vec::new(),
                notes: Vec::new(),
                raw_output: String::new(),
            });
        }
    }

    // "warning: ..." without a code
    if line.starts_with("warning:") {
        let message = line[8..].trim();
        if !message.is_empty() {
            return Some(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                code: None,
                message: message.to_string(),
                location: None,
                related_locations: Vec::new(),
                suggestions: Vec::new(),
                notes: Vec::new(),
                raw_output: String::new(),
            });
        }
    }

    None
}

/// Parse Rust location line: "--> src/main.rs:10:5"
fn parse_rust_location(line: &str) -> Option<DiagnosticLocation> {
    let line = line.trim().trim_start_matches("--> ");
    let parts: Vec<&str> = line.split(':').collect();

    if parts.len() >= 2 {
        let file = parts[0].to_string();
        let line_num = parts[1].parse::<u32>().ok()?;
        let column = parts.get(2).and_then(|c| c.parse::<u32>().ok());

        return Some(DiagnosticLocation {
            file,
            line: line_num,
            column,
            end_line: None,
            end_column: None,
            snippet: None,
        });
    }

    None
}

/// Parse TypeScript compiler output.
fn parse_typescript_output(output: &str, report: &mut DiagnosticReport) {
    for line in output.lines() {
        // "src/index.ts(10,5): error TS2304: Cannot find name 'foo'."
        if let Some(diag) = parse_typescript_diagnostic_line(line) {
            match diag.severity {
                DiagnosticSeverity::Error => report.error_count += 1,
                DiagnosticSeverity::Warning => report.warning_count += 1,
                _ => {}
            }
            report.diagnostics.push(diag);
        }
    }
}

/// Parse a single TypeScript diagnostic line.
fn parse_typescript_diagnostic_line(line: &str) -> Option<Diagnostic> {
    // "src/index.ts(10,5): error TS2304: Cannot find name 'foo'."
    let line = line.trim();

    // Find the location part: "file(line,col):"
    let paren_start = line.find('(')?;
    let paren_end = line.find(')')?;
    let colon_after_paren = line[paren_end..].find(':')? + paren_end;

    let file = &line[..paren_start];
    let loc_str = &line[paren_start + 1..paren_end];
    let rest = &line[colon_after_paren + 1..].trim();

    // Parse line,col
    let loc_parts: Vec<&str> = loc_str.split(',').collect();
    let line_num = loc_parts.first()?.parse::<u32>().ok()?;
    let column = loc_parts.get(1).and_then(|c| c.parse::<u32>().ok());

    // Parse "error TSxxxx: message"
    let (severity, code, message) = if rest.starts_with("error TS") {
        let ts_end = rest.find(':')?;
        let code = rest[6..ts_end].to_string();
        let msg = rest[ts_end + 1..].trim().to_string();
        (DiagnosticSeverity::Error, Some(code), msg)
    } else if rest.starts_with("warning TS") {
        let ts_end = rest.find(':')?;
        let code = rest[8..ts_end].to_string();
        let msg = rest[ts_end + 1..].trim().to_string();
        (DiagnosticSeverity::Warning, Some(code), msg)
    } else {
        return None;
    };

    Some(Diagnostic {
        severity,
        code,
        message,
        location: Some(DiagnosticLocation {
            file: file.to_string(),
            line: line_num,
            column,
            end_line: None,
            end_column: None,
            snippet: None,
        }),
        related_locations: Vec::new(),
        suggestions: Vec::new(),
        notes: Vec::new(),
        raw_output: line.to_string(),
    })
}

/// Parse Go compiler output.
fn parse_go_output(output: &str, report: &mut DiagnosticReport) {
    for line in output.lines() {
        // "main.go:10:5: undefined: foo"
        if let Some(diag) = parse_go_diagnostic_line(line) {
            report.error_count += 1;
            report.diagnostics.push(diag);
        }
    }
}

/// Parse a single Go diagnostic line.
fn parse_go_diagnostic_line(line: &str) -> Option<Diagnostic> {
    let line = line.trim();
    let parts: Vec<&str> = line.splitn(4, ':').collect();

    if parts.len() >= 4 && parts[0].ends_with(".go") {
        let file = parts[0].to_string();
        let line_num = parts[1].parse::<u32>().ok()?;
        let column = parts[2].parse::<u32>().ok();
        let message = parts[3].trim().to_string();

        return Some(Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: None,
            message,
            location: Some(DiagnosticLocation {
                file,
                line: line_num,
                column,
                end_line: None,
                end_column: None,
                snippet: None,
            }),
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: line.to_string(),
        });
    }

    None
}

/// Parse generic compiler output (fallback).
fn parse_generic_output(output: &str, report: &mut DiagnosticReport) {
    for line in output.lines() {
        let line = line.trim();

        // Try to find "error:" or "Error:" anywhere
        if line.to_lowercase().contains("error") {
            let message = line.to_string();
            report.error_count += 1;
            report.diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Error,
                code: None,
                message,
                location: None,
                related_locations: Vec::new(),
                suggestions: Vec::new(),
                notes: Vec::new(),
                raw_output: line.to_string(),
            });
        } else if line.to_lowercase().contains("warning") {
            let message = line.to_string();
            report.warning_count += 1;
            report.diagnostics.push(Diagnostic {
                severity: DiagnosticSeverity::Warning,
                code: None,
                message,
                location: None,
                related_locations: Vec::new(),
                suggestions: Vec::new(),
                notes: Vec::new(),
                raw_output: line.to_string(),
            });
        }
    }
}

/// Extract structured fix information from a diagnostic.
///
/// Returns a tuple of (fix_type, target_file, suggested_change).
pub fn extract_fix_info(diagnostic: &Diagnostic) -> Option<FixInfo> {
    let code = diagnostic.code.as_deref().unwrap_or("");
    let message = &diagnostic.message;
    let lower_message = message.to_lowercase();

    // Rust error codes
    match code {
        // E0433: unresolved import
        "E0433" => {
            if let Some(module) = extract_quoted_name(message) {
                let suggested_change = format!("Add import for `{}`", module);
                return Some(FixInfo {
                    fix_type: FixType::AddImport,
                    target_file: diagnostic.file_path().map(String::from),
                    target_item: Some(module),
                    suggested_change,
                });
            }
        }

        // E0425: cannot find value in this scope
        "E0425" => {
            if let Some(name) = extract_quoted_name(message) {
                let suggested_change = format!("Add import or declaration for `{}`", name);
                return Some(FixInfo {
                    fix_type: FixType::AddImport,
                    target_file: diagnostic.file_path().map(String::from),
                    target_item: Some(name),
                    suggested_change,
                });
            }
        }

        // E0463: can't find crate
        "E0463" => {
            if let Some(crate_name) = extract_quoted_name(message) {
                return Some(FixInfo {
                    fix_type: FixType::AddDependency,
                    target_file: Some("Cargo.toml".to_string()),
                    target_item: Some(crate_name),
                    suggested_change: format!("Add dependency to Cargo.toml"),
                });
            }
        }

        // E0412: cannot find type in this scope
        "E0412" => {
            if let Some(type_name) = extract_quoted_name(message) {
                return Some(FixInfo {
                    fix_type: FixType::AddImport,
                    target_file: diagnostic.file_path().map(String::from),
                    target_item: Some(type_name),
                    suggested_change: format!("Add import for type"),
                });
            }
        }

        // E0308: mismatched types
        "E0308" => {
            return Some(FixInfo {
                fix_type: FixType::FixType,
                target_file: diagnostic.file_path().map(String::from),
                target_item: None,
                suggested_change: "Fix type mismatch".to_string(),
            });
        }

        _ => {}
    }

    // Pattern matching for messages without codes
    if lower_message.contains("cannot find crate")
        || lower_message.contains("can't find crate")
        || lower_message.contains("unresolved import")
    {
        let crate_name = extract_quoted_name(message);
        return Some(FixInfo {
            fix_type: FixType::AddDependency,
            target_file: Some("Cargo.toml".to_string()),
            target_item: crate_name,
            suggested_change: "Add missing dependency".to_string(),
        });
    }

    if lower_message.contains("cannot find") && lower_message.contains("in this scope") {
        let name = extract_quoted_name(message);
        return Some(FixInfo {
            fix_type: FixType::AddImport,
            target_file: diagnostic.file_path().map(String::from),
            target_item: name,
            suggested_change: "Add missing import".to_string(),
        });
    }

    if lower_message.contains("mismatched types") || lower_message.contains("type mismatch") {
        return Some(FixInfo {
            fix_type: FixType::FixType,
            target_file: diagnostic.file_path().map(String::from),
            target_item: None,
            suggested_change: "Fix type mismatch".to_string(),
        });
    }

    None
}

/// Information about how to fix a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixInfo {
    /// The type of fix needed.
    pub fix_type: FixType,

    /// The file that needs to be modified.
    pub target_file: Option<String>,

    /// The specific item (import, dependency, etc.) to add/modify.
    pub target_item: Option<String>,

    /// Human-readable description of the suggested change.
    pub suggested_change: String,
}

/// Types of fixes that can be applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FixType {
    /// Add a missing import statement.
    AddImport,
    /// Add a missing dependency to manifest.
    AddDependency,
    /// Fix a type error (conversion, annotation).
    FixType,
    /// Fix a syntax error.
    FixSyntax,
}

impl std::fmt::Display for FixType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FixType::AddImport => write!(f, "add_import"),
            FixType::AddDependency => write!(f, "add_dependency"),
            FixType::FixType => write!(f, "fix_type"),
            FixType::FixSyntax => write!(f, "fix_syntax"),
        }
    }
}

/// Extract a quoted name from an error message.
fn extract_quoted_name(message: &str) -> Option<String> {
    // Look for backtick-quoted names: `foo`
    if let Some(start) = message.find('`') {
        if let Some(end) = message[start + 1..].find('`') {
            let name = &message[start + 1..start + 1 + end];
            if !name.is_empty() {
                return Some(name.to_string());
            }
        }
    }

    // Look for single-quoted names: 'foo'
    if let Some(start) = message.find('\'') {
        if let Some(end) = message[start + 1..].find('\'') {
            let name = &message[start + 1..start + 1 + end];
            if !name.is_empty() && !name.contains(' ') {
                return Some(name.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    // ============== Compiler Detection Tests ==============

    #[test]
    fn test_detect_rust_compiler_with_error_code() {
        let output = "error[E0463]: can't find crate for `serde_json`";
        assert_eq!(detect_compiler(output), CompilerType::Rust);
    }

    #[test]
    fn test_detect_rust_compiler_with_arrow() {
        let output = "error: something\n --> src/main.rs:10:5\n   |";
        assert_eq!(detect_compiler(output), CompilerType::Rust);
    }

    #[test]
    fn test_detect_typescript_compiler() {
        let output = "src/index.ts(10,5): error TS2304: Cannot find name 'foo'.";
        assert_eq!(detect_compiler(output), CompilerType::TypeScript);
    }

    #[test]
    fn test_detect_go_compiler() {
        let output = "main.go:10:5: undefined: foo";
        assert_eq!(detect_compiler(output), CompilerType::Go);
    }

    #[test]
    fn test_detect_unknown_compiler() {
        let output = "something went wrong";
        assert_eq!(detect_compiler(output), CompilerType::Unknown);
    }

    // ============== Rust Parsing Tests ==============

    #[test]
    fn test_parse_rust_error_with_code() {
        let output = "error[E0463]: can't find crate for `serde_json`\n --> src/main.rs:1:5";
        let report = parse_compiler_output(output);

        assert_eq!(report.compiler, CompilerType::Rust);
        assert_eq!(report.error_count, 1);
        assert_eq!(report.diagnostics.len(), 1);

        let diag = &report.diagnostics[0];
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code, Some("E0463".to_string()));
        assert!(diag.message.contains("can't find crate"));
        assert!(diag.has_location());
        assert_eq!(diag.file_path(), Some("src/main.rs"));
        assert_eq!(diag.line(), Some(1));
        assert_eq!(diag.column(), Some(5));
    }

    #[test]
    fn test_parse_rust_error_without_code() {
        let output = "error: could not compile `myproject`";
        let report = parse_compiler_output(output);

        assert_eq!(report.error_count, 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert!(diag.code.is_none());
    }

    #[test]
    fn test_parse_rust_warning() {
        let output = "warning[E0599]: no method named `foo` found\n --> src/lib.rs:20:10";
        let report = parse_compiler_output(output);

        assert_eq!(report.warning_count, 1);
        let diag = &report.diagnostics[0];
        assert_eq!(diag.severity, DiagnosticSeverity::Warning);
        assert_eq!(diag.code, Some("E0599".to_string()));
    }

    #[test]
    fn test_parse_rust_with_help() {
        let output = r#"error[E0425]: cannot find value `HashMap` in this scope
 --> src/main.rs:5:9
  |
5 |     let map = HashMap::new();
  |               ^^^^^^^ not found in this scope
  |
help: consider importing this struct
  |
1 | use std::collections::HashMap;
  |"#;

        let report = parse_compiler_output(output);

        assert_eq!(report.error_count, 1);
        let diag = &report.diagnostics[0];
        assert!(!diag.suggestions.is_empty());
        assert!(diag.suggestions[0].message.contains("consider importing"));
    }

    #[test]
    fn test_parse_rust_with_note() {
        let output = r#"error[E0308]: mismatched types
 --> src/main.rs:10:5
  |
  = note: expected type `&str`
             found type `String`"#;

        let report = parse_compiler_output(output);

        let diag = &report.diagnostics[0];
        assert!(!diag.notes.is_empty());
        assert!(diag.notes[0].contains("expected type"));
    }

    #[test]
    fn test_parse_rust_multiple_errors() {
        let output = r#"error[E0463]: can't find crate for `serde`
 --> src/main.rs:1:1
error[E0463]: can't find crate for `tokio`
 --> src/main.rs:2:1"#;

        let report = parse_compiler_output(output);

        assert_eq!(report.error_count, 2);
        assert_eq!(report.diagnostics.len(), 2);
    }

    // ============== TypeScript Parsing Tests ==============

    #[test]
    fn test_parse_typescript_error() {
        let output = "src/index.ts(10,5): error TS2304: Cannot find name 'foo'.";
        let report = parse_compiler_output(output);

        assert_eq!(report.compiler, CompilerType::TypeScript);
        assert_eq!(report.error_count, 1);

        let diag = &report.diagnostics[0];
        assert_eq!(diag.severity, DiagnosticSeverity::Error);
        assert_eq!(diag.code, Some("TS2304".to_string()));
        assert!(diag.message.contains("Cannot find name"));
        assert_eq!(diag.file_path(), Some("src/index.ts"));
        assert_eq!(diag.line(), Some(10));
        assert_eq!(diag.column(), Some(5));
    }

    #[test]
    fn test_parse_typescript_multiple_errors() {
        let output = r#"src/a.ts(1,1): error TS2304: Cannot find name 'a'.
src/b.ts(2,2): error TS2304: Cannot find name 'b'."#;

        let report = parse_compiler_output(output);
        assert_eq!(report.error_count, 2);
    }

    // ============== Go Parsing Tests ==============

    #[test]
    fn test_parse_go_error() {
        let output = "main.go:10:5: undefined: foo";
        let report = parse_compiler_output(output);

        assert_eq!(report.compiler, CompilerType::Go);
        assert_eq!(report.error_count, 1);

        let diag = &report.diagnostics[0];
        assert_eq!(diag.file_path(), Some("main.go"));
        assert_eq!(diag.line(), Some(10));
        assert_eq!(diag.column(), Some(5));
        assert!(diag.message.contains("undefined"));
    }

    // ============== Generic Parsing Tests ==============

    #[test]
    fn test_parse_generic_error() {
        let output = "Error: something went wrong";
        let report = parse_compiler_output(output);

        assert_eq!(report.compiler, CompilerType::Unknown);
        assert_eq!(report.error_count, 1);
    }

    #[test]
    fn test_parse_generic_warning() {
        let output = "Warning: deprecated function";
        let report = parse_compiler_output(output);

        assert_eq!(report.warning_count, 1);
    }

    // ============== Fix Info Extraction Tests ==============

    #[test]
    fn test_extract_fix_info_missing_crate() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("E0463".to_string()),
            message: "can't find crate for `serde_json`".to_string(),
            location: Some(DiagnosticLocation::new("src/main.rs", 1)),
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        };

        let fix_info = extract_fix_info(&diag).unwrap();
        assert_eq!(fix_info.fix_type, FixType::AddDependency);
        assert_eq!(fix_info.target_file, Some("Cargo.toml".to_string()));
        assert_eq!(fix_info.target_item, Some("serde_json".to_string()));
    }

    #[test]
    fn test_extract_fix_info_missing_import() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("E0425".to_string()),
            message: "cannot find value `HashMap` in this scope".to_string(),
            location: Some(DiagnosticLocation::new("src/main.rs", 5)),
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        };

        let fix_info = extract_fix_info(&diag).unwrap();
        assert_eq!(fix_info.fix_type, FixType::AddImport);
        assert_eq!(fix_info.target_file, Some("src/main.rs".to_string()));
        assert_eq!(fix_info.target_item, Some("HashMap".to_string()));
    }

    #[test]
    fn test_extract_fix_info_type_mismatch() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("E0308".to_string()),
            message: "mismatched types: expected `&str`, found `String`".to_string(),
            location: Some(DiagnosticLocation::new("src/main.rs", 10)),
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        };

        let fix_info = extract_fix_info(&diag).unwrap();
        assert_eq!(fix_info.fix_type, FixType::FixType);
    }

    #[test]
    fn test_extract_fix_info_without_code() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: None,
            message: "cannot find crate for `tokio`".to_string(),
            location: None,
            related_locations: Vec::new(),
            suggestions: Vec::new(),
            notes: Vec::new(),
            raw_output: String::new(),
        };

        let fix_info = extract_fix_info(&diag).unwrap();
        assert_eq!(fix_info.fix_type, FixType::AddDependency);
    }

    // ============== Helper Function Tests ==============

    #[test]
    fn test_extract_quoted_name_backticks() {
        assert_eq!(
            extract_quoted_name("can't find crate `serde_json`"),
            Some("serde_json".to_string())
        );
    }

    #[test]
    fn test_extract_quoted_name_single_quotes() {
        assert_eq!(
            extract_quoted_name("cannot find 'HashMap'"),
            Some("HashMap".to_string())
        );
    }

    #[test]
    fn test_extract_quoted_name_none() {
        assert_eq!(extract_quoted_name("some error message"), None);
    }

    // ============== DiagnosticLocation Tests ==============

    #[test]
    fn test_diagnostic_location_format_short() {
        let loc = DiagnosticLocation::with_column("src/main.rs", 10, 5);
        assert_eq!(loc.format_short(), "src/main.rs:10:5");

        let loc_no_col = DiagnosticLocation::new("src/lib.rs", 20);
        assert_eq!(loc_no_col.format_short(), "src/lib.rs:20");
    }

    #[test]
    fn test_diagnostic_location_display() {
        let loc = DiagnosticLocation::with_column("src/main.rs", 10, 5);
        assert_eq!(format!("{}", loc), "src/main.rs:10:5");
    }

    // ============== DiagnosticReport Tests ==============

    #[test]
    fn test_diagnostic_report_by_file() {
        let output = r#"error[E0463]: can't find crate for `serde`
 --> src/main.rs:1:1
error[E0463]: can't find crate for `tokio`
 --> src/main.rs:2:1
error[E0463]: can't find crate for `reqwest`
 --> src/lib.rs:1:1"#;

        let report = parse_compiler_output(output);
        let by_file = report.by_file();

        assert_eq!(by_file.get("src/main.rs").map(|v| v.len()), Some(2));
        assert_eq!(by_file.get("src/lib.rs").map(|v| v.len()), Some(1));
    }

    #[test]
    fn test_diagnostic_report_iterators() {
        let output = r#"error[E0463]: can't find crate
 --> src/main.rs:1:1
warning: unused variable
 --> src/main.rs:2:1"#;

        let report = parse_compiler_output(output);

        assert_eq!(report.errors().count(), 1);
        assert_eq!(report.warnings().count(), 1);
    }

    // ============== Display Tests ==============

    #[test]
    fn test_severity_display() {
        assert_eq!(format!("{}", DiagnosticSeverity::Error), "error");
        assert_eq!(format!("{}", DiagnosticSeverity::Warning), "warning");
        assert_eq!(format!("{}", DiagnosticSeverity::Note), "note");
        assert_eq!(format!("{}", DiagnosticSeverity::Help), "help");
    }

    #[test]
    fn test_compiler_type_display() {
        assert_eq!(format!("{}", CompilerType::Rust), "rust");
        assert_eq!(format!("{}", CompilerType::TypeScript), "typescript");
        assert_eq!(format!("{}", CompilerType::Go), "go");
        assert_eq!(format!("{}", CompilerType::Unknown), "unknown");
    }

    #[test]
    fn test_fix_type_display() {
        assert_eq!(format!("{}", FixType::AddImport), "add_import");
        assert_eq!(format!("{}", FixType::AddDependency), "add_dependency");
        assert_eq!(format!("{}", FixType::FixType), "fix_type");
        assert_eq!(format!("{}", FixType::FixSyntax), "fix_syntax");
    }

    // ============== Diagnostic Methods Tests ==============

    #[test]
    fn test_diagnostic_accessors() {
        let diag = Diagnostic::new("test error");
        assert!(!diag.has_code());
        assert!(!diag.has_location());
        assert!(!diag.has_suggestions());
        assert!(diag.file_path().is_none());
        assert!(diag.line().is_none());
        assert!(diag.column().is_none());
    }

    #[test]
    fn test_diagnostic_with_all_fields() {
        let diag = Diagnostic {
            severity: DiagnosticSeverity::Error,
            code: Some("E0001".to_string()),
            message: "test".to_string(),
            location: Some(DiagnosticLocation::with_column("test.rs", 1, 1)),
            related_locations: Vec::new(),
            suggestions: vec![DiagnosticSuggestion::new("fix it")],
            notes: vec!["note".to_string()],
            raw_output: String::new(),
        };

        assert!(diag.has_code());
        assert!(diag.has_location());
        assert!(diag.has_suggestions());
        assert_eq!(diag.file_path(), Some("test.rs"));
        assert_eq!(diag.line(), Some(1));
        assert_eq!(diag.column(), Some(1));
    }

    // ============== DiagnosticSuggestion Tests ==============

    #[test]
    fn test_suggestion_with_replacement() {
        let loc = DiagnosticLocation::new("test.rs", 10);
        let suggestion = DiagnosticSuggestion::with_replacement(
            "add import",
            loc,
            "use std::collections::HashMap;",
        );

        assert!(suggestion.is_applicable);
        assert!(suggestion.location.is_some());
        assert!(suggestion.replacement.is_some());
    }
}
