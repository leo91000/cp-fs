use arboard::Clipboard;
use clap::Parser;
use content_inspector::{inspect, ContentType};
use glob::Pattern;
use ignore::Walk;
use std::collections::{HashSet, VecDeque};
use std::fs;
use std::path::PathBuf;
use std::thread::sleep;
use std::time::Duration;

const IGNORED_FILES: &[&str] = &[
    "yarn.lock",
    "Cargo.lock",
    "pnpm-lock.yaml",
    "package-lock.json",
    ".DS_Store",
    "thumbs.db",
    ".env",
    ".env.local",
    ".env.development",
    ".env.production",
    "node_modules",
    "target",
    "dist",
    "build",
    "LICENSE.md",
    "LICENSE",
];

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory to process
    path: PathBuf,

    /// Additional files or directories to ignore (supports glob patterns)
    #[arg(short = 'i', long = "ignore", value_delimiter = ',')]
    ignore: Vec<String>,
}

fn is_text_file(content: &[u8]) -> bool {
    matches!(
        inspect(content),
        ContentType::UTF_8 | ContentType::UTF_16LE | ContentType::UTF_16BE
    )
}

struct IgnorePatterns {
    exact_matches: HashSet<String>,
    glob_patterns: Vec<Pattern>,
}

impl IgnorePatterns {
    fn new() -> Self {
        Self {
            exact_matches: HashSet::new(),
            glob_patterns: Vec::new(),
        }
    }

    fn add_pattern(&mut self, pattern: &str) {
        // If the pattern contains glob characters, compile it as a glob pattern
        if pattern.contains('*') || pattern.contains('?') || pattern.contains('[') {
            match Pattern::new(pattern) {
                Ok(glob_pattern) => self.glob_patterns.push(glob_pattern),
                Err(e) => eprintln!("Invalid glob pattern '{}': {}", pattern, e),
            }
        } else {
            // Otherwise, treat it as an exact match
            self.exact_matches.insert(pattern.to_string());
        }
    }

    fn should_ignore(&self, path_str: &str) -> bool {
        // Check for exact matches
        if self.exact_matches.contains(path_str) {
            return true;
        }

        // Check against glob patterns
        for pattern in &self.glob_patterns {
            if pattern.matches(path_str) {
                return true;
            }
        }

        false
    }
}

fn should_ignore_file(
    path: &std::path::Path,
    base_path: &std::path::Path,
    ignore_patterns: &IgnorePatterns,
) -> bool {
    // Check if the file name matches any ignored pattern
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if ignore_patterns.should_ignore(file_name) {
            return true;
        }
    }

    // Check if any parent directory matches ignored patterns
    for ancestor in path.ancestors() {
        if ancestor == base_path {
            break;
        }
        if let Some(dir_name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if ignore_patterns.should_ignore(dir_name) {
                return true;
            }
        }
    }

    // Check if the relative path matches any ignored pattern
    if let Ok(relative_path) = path.strip_prefix(base_path) {
        let relative_path_str = relative_path.to_string_lossy();
        if ignore_patterns.should_ignore(&relative_path_str) {
            return true;
        }

        // Also check path components for glob matches
        let mut current = PathBuf::new();
        let components: VecDeque<_> = relative_path.components().collect();
        for component in components {
            current.push(component);
            let current_str = current.to_string_lossy();
            if ignore_patterns.should_ignore(&current_str) {
                return true;
            }
        }
    }

    false
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut output = String::new();

    // Set up ignore patterns
    let mut ignore_patterns = IgnorePatterns::new();

    // Add default ignored files
    for &file in IGNORED_FILES {
        ignore_patterns.add_pattern(file);
    }

    // Add user-provided ignored files
    for pattern in &args.ignore {
        ignore_patterns.add_pattern(pattern);
    }

    let mut files = HashSet::new();

    // Walk through directory respecting gitignore
    for entry in Walk::new(&args.path) {
        let entry = match entry {
            Ok(entry) => entry,
            Err(err) => {
                eprintln!("Error accessing entry: {}", err);
                continue;
            }
        };

        // Skip if it's not a file
        if !entry.file_type().map_or(false, |ft| ft.is_file()) {
            continue;
        }

        if should_ignore_file(entry.path(), &args.path, &ignore_patterns) {
            continue;
        }

        // Read file content
        let content = match fs::read(entry.path()) {
            Ok(content) => content,
            Err(err) => {
                eprintln!("Error reading file {}: {}", entry.path().display(), err);
                continue;
            }
        };

        // Skip if not a text file
        if !is_text_file(&content) {
            continue;
        }

        // Convert content to string
        let content_str = match String::from_utf8_lossy(&content) {
            std::borrow::Cow::Borrowed(s) => s.to_string(),
            std::borrow::Cow::Owned(s) => s,
        };

        // Get relative path
        let relative_path = match entry.path().strip_prefix(&args.path) {
            Ok(path) => path.to_string_lossy().into_owned(),
            Err(err) => {
                eprintln!("Error getting relative path: {}", err);
                continue;
            }
        };

        // Format output
        output.push_str("---\n");
        output.push_str(&format!("file: {}\n", relative_path));
        output.push_str("---\n\n");
        output.push_str(&content_str);
        output.push_str("\n\n");
        files.insert(relative_path);
    }

    // Copy to clipboard
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(&output)?;
    sleep(Duration::from_millis(100));

    println!("File structure and contents copied to clipboard:");
    for file in files {
        println!("- {}", file);
    }

    Ok(())
}
