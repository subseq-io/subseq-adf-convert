pub mod adf;
pub mod convert;
pub mod html;

use crate::html::ADFBuilder;
use html5ever::tendril::Tendril;
use html5ever::tokenizer::{BufferQueue, Tokenizer, TokenizerOpts};

fn main() {
    let html = r#"
        <p>Paragraph with <br /> line break.</p>
        <pre><code>let x = 42;
println!("Hello");</code></pre>
    "#;

    let mut queue: BufferQueue = Default::default();
    queue.push_back(Tendril::from_slice(html));

    let builder = ADFBuilder::new();
    let tok = Tokenizer::new(builder, TokenizerOpts::default());

    while !queue.is_empty() {
        let _ = tok.feed(&mut queue);
    }
    tok.end();

    let adf_tree = tok.sink.emit();
    println!("{:#?}", adf_tree);
}
