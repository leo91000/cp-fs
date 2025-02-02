use clap::Parser;
use clipboard::{ClipboardContext, ClipboardProvider};
use content_inspector::{inspect, ContentType};
use ignore::Walk;
use std::fs;
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Path to the directory to process
    #[arg(short, long)]
    path: PathBuf,
}

fn is_text_file(content: &[u8]) -> bool {
    matches!(
        inspect(content),
        ContentType::UTF_8 | ContentType::UTF_16LE | ContentType::UTF_16BE
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let mut output = String::new();

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
    }

    // Copy to clipboard
    let mut ctx: ClipboardContext = ClipboardProvider::new()?;
    ctx.set_contents(output.clone())?;

    println!("File structure and contents copied to clipboard:");
    println!("{}", output);

    Ok(())
}
