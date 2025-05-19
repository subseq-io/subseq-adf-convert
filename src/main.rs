use serde_json::{Value, from_value};
use std::env;
use std::fs;
use subseq_adf_convert::adf_to_html::adf_to_html;
use subseq_adf_convert::markdown::html_to_markdown;
use subseq_adf_convert::markdown::markdown_to_adf;

use subseq_adf_convert::adf::adf_types::AdfBlockNode;

fn main() {
    // Get the first argument (after the program name)
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <input_file.json>", args[0]);
        std::process::exit(1);
    }

    let filename = &args[1];

    // Read file contents
    let contents = fs::read_to_string(filename).unwrap_or_else(|err| {
        eprintln!("Failed to read file {}: {}", filename, err);
        std::process::exit(1);
    });

    // Parse as JSON
    let json: Value = serde_json::from_str(&contents).unwrap_or_else(|err| {
        eprintln!("Invalid JSON: {}", err);
        std::process::exit(1);
    });

    // Extract "description" field
    let fields = json.get("fields").unwrap_or_else(|| {
        eprintln!("Missing 'fields' field in JSON");
        std::process::exit(1);
    });

    let description = fields.get("description").cloned().unwrap_or_else(|| {
        eprintln!("Missing 'description' field in fields");
        std::process::exit(1);
    });

    // Parse as AdfNode
    let adf: AdfBlockNode = from_value(description).unwrap_or_else(|err| {
        eprintln!("Failed to parse 'description' as AdfNode: {}", err);
        std::process::exit(1);
    });
    let html = adf_to_html(vec![adf], &contents);
    println!("\n--- HTML ---\n{}\n---      ---\n", html);
    let markdown = html_to_markdown(html);
    println!("\n--- MARKDOWN ---\n{}\n---          ---\n", markdown);

    let adf = markdown_to_adf(&markdown);
    let adf = adf.unwrap_or_else(|| {
        eprintln!("Failed to convert markdown to AdfNode");
        std::process::exit(1);
    });
    eprintln!("{}", serde_json::to_string(&adf).unwrap());
}
