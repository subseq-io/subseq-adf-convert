# Subseq ADF Convert

version: 0.1.0 (first release)

This library provides **lossless, round-trippable conversion between Atlassian
Document Format (ADF), HTML, and Markdown**, with strict fidelity to both the
ADF specification and predictable Markdown generation.

It is designed as a **strictly deterministic parser and renderer**, capable of
roundtripping:


## Key Features

- ✅ **ADF to HTML rendering**
- ✅ **HTML to ADF parsing**
- ✅ **ADF to Markdown conversion with custom handlers for complex edge cases (tables, media, expand, status, etc.)**
- ✅ **Markdown to ADF conversion via HTML bridging using `markdown-rs` and `html5ever`**
- ✅ **Custom handlers for ensuring consistency in tables, inline marks, and media rendering**
- ✅ **Robust HTML sanitizer post-processing to correct structural errors from Markdown parsers**

## Design Philosophy

- **ADF-first model:** ADF is the authoritative data model. HTML and Markdown are considered serialization formats only.
- **Roundtrip Guarantee:** All generated ADF must survive a full conversion cycle back to itself. Compromises are only accepted where Markdown format limitations make fidelity impossible.
- **Markdown as supported input:**  
  Valid, standards-compliant Markdown is supported as input and will be parsed to ADF. However, **features unique to ADF (e.g., mentions, emojis, statuses, inline cards) require either our tools or plugins to be produced and cannot be manually authored using standard Markdown syntax.**  
  Users are expected to edit the **standard Markdown representation**—we will correctly handle supported Markdown, and enhanced ADF features will be retained if present in the source ADF or inserted by compatible tooling.
- **Clean sanitization:** HTML produced by `markdown-rs` or third-party tools is automatically sanitized (e.g., nested `<a>` tags are flattened, invalid `<p>` wrappers are removed) to ensure valid roundtrip.
- **Predictable fallback behavior:** Where Markdown cannot express ADF features (e.g., task lists with state, decision lists), we will emit valid HTML blocks using custom tags.

## Architecture

| From       | To          | Notes                                              |
|------------|-------------|----------------------------------------------------|
| ADF        | HTML        | Fully custom renderer ensuring semantic correctness |
| HTML       | Markdown    | Uses `htmd` with extended element handlers        |
| Markdown   | HTML        | Uses `markdown-rs`, followed by HTML5ever sanitization |
| HTML       | ADF         | Custom walker using `html5ever::RcDom`            |

### Special Handling
- **Tables:** Custom renderer ensuring valid GFM tables with inline formatting and proper escaping.
- **Media:** Media nodes are wrapped and validated with link marks when necessary.
- **Expand/Details:** Uses `<details><summary>` and renders as block-level Markdown extensions.
- **Inline complexity:** Inline emojis, mentions, statuses, links, and inline cards are supported with proper mark serialization.

## Known Limitations
- **Standard Markdown input is supported where valid.**  
  Features specific to ADF cannot be expressed directly in standard Markdown without using custom syntax or extensions (e.g., `<adf-status>`, `<adf-emoji>`).
- **Roundtrip guarantees are scoped to documents created via our rendering pipeline.**
- **Malformed or ambiguous user-authored Markdown outside of GFM or commonmark standards is not supported.**
- **Sanitizer ensures HTML correctness but does not attempt to repair semantically invalid Markdown.**

## Test Coverage

### Roundtrip Tests
- ✅ Simple paragraphs and headings
- ✅ Mixed inline elements (emoji, mention, status, links)
- ✅ Decision and task lists with accurate state restoration
- ✅ Complex tables with inline formatting, emoji, and links
- ✅ Expand and nested expand blocks
- ✅ Full document structures with media, tables, blockquotes, and rules

### Sanitizer Tests
- ✅ Flattening nested `<a>` inside `<a>`
- ✅ Removing invalid `<p>` wrappers around block elements
- ✅ Enforcing valid HTML output structure prior to ADF parsing

## Usage Example

```rust
use your_crate::html_to_adf::html_to_adf;
use your_crate::adf_to_html::adf_to_html;
use your_crate::markdown::{adf_to_markdown, markdown_to_adf};

// ADF -> HTML -> Markdown -> HTML -> ADF
let adf = my_custom_adf();
let html = adf_to_html(vec![adf.clone()]);
let markdown = adf_to_markdown(&[adf.clone()]);
let html_from_md = markdown_to_html(&markdown);
let adf_back = html_to_adf(&html_from_md);

assert_eq!(adf_back, adf);

