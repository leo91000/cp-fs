use arboard::Clipboard;
use clap::Parser;
use content_inspector::{inspect, ContentType};
use ignore::Walk;
use std::collections::HashSet;
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
}

fn is_text_file(content: &[u8]) -> bool {
    matches!(
        inspect(content),
        ContentType::UTF_8 | ContentType::UTF_16LE | ContentType::UTF_16BE
    )
}

fn should_ignore_file(path: &std::path::Path, ignored_files: &HashSet<&str>) -> bool {
    // Check if the file name matches any ignored file
    if let Some(file_name) = path.file_name().and_then(|n| n.to_str()) {
        if ignored_files.contains(file_name) {
            return true;
        }
    }

    // Check if any parent directory matches ignored files
    for ancestor in path.ancestors() {
        if let Some(dir_name) = ancestor.file_name().and_then(|n| n.to_str()) {
            if ignored_files.contains(dir_name) {
                return true;
            }
        }
    }

    false
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut output = String::new();

    let ignored_files = HashSet::from_iter(IGNORED_FILES.iter().copied());
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

        if should_ignore_file(entry.path(), &ignored_files) {
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
