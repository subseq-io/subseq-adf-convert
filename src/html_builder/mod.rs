/*!
This is a vendored copy of the [`html_builder`](https://crates.io/crates/html_builder) crate.
We have to vend it because the original crate is not maintained anymore and there are defects in
its behavior.  See the `README.md` file for more information.

[`Buffer`] is a magic text buffer that makes writing HTML pleasant.

Here's a teaser of what it looks like in use:

```
use subseq_adf_convert::html_builder::*;                          // Contents added to buf by each statement
use std::fmt::Write;                          // ---------------------------------------
                                              //
let mut buf = Buffer::new();                  //
let mut html = buf.html().attr("lang='en'");  // <html lang='en'>
writeln!(html.head().title(), "Title!")?;     // <head><title>Title!
writeln!(html.body().h1(), "Header!")?;       // </title></head><body><h1>Header!
buf.finish()                                  // </h1></body></html>
# ; Ok::<(), std::fmt::Error>(())
```

## Longer example

```
use subseq_adf_convert::html_builder::*;
use std::fmt::Write;

// Start by creating a Buffer.  This contains a text buffer that we're going
// to be writing into.
let mut buf = Buffer::new();

// The Html5 trait provides various helper methods.  For instance, doctype()
// simply writes the <!DOCTYPE html> header
buf.doctype();

// Most helper methods create child nodes.  You can set a node's attributes
// like so
let mut html = buf.html().attr("lang='en'");

let mut head = html.head();

// Just like Buffer, nodes are also writable.  Set their contents by
// writing into them.
writeln!(head.title(), "Website!")?;

// Meta is a "void element", meaning it doesn't need a closing tag.  This is
// handled correctly.
head.meta().attr("charset='utf-8'");

let mut body = html.body();
writeln!(body.h1(), "It's a website!")?;

// Generating HTML in a loop
let mut list = body.ul();
for i in 1..=3 {
    writeln!(
        list.li().a().attr(
            &format!("href='/page_{}.html'", i)
        ),
        "Page {}", i,
    )?
}

// You can write functions which add subtrees to a node
fn figure_with_caption(parent: &mut Node, src: &str, cap: &str) {
    let mut fig = parent.figure();
    fig.img()
        .attr(&format!("src='{}'", src))
        .attr(&format!("alt='{}'", cap));
    writeln!(fig.figcaption(), "{}", cap).unwrap();
}

figure_with_caption(&mut body, "img.jpg", "Awesome image");

// Text contents in an inner node
let mut footer = body.footer();
writeln!(footer, "Last modified")?;
writeln!(footer.time(), "2021-04-12")?;

// We also provide a kind of pseudo-node for writing comments
write!(body.comment(), "Thanks for reading")?;

// Finally, call finish() to extract the buffer.
buf.finish()
# ; Ok::<(), std::fmt::Error>(())
```
```html
<!DOCTYPE html>
<html lang='en'>
 <head>
  <title>Website!</title>
  <meta charset='utf-8'>
 </head>
 <body>
  <h1>It's a website!</h1>
  <ul>
   <li><a href='/page_1.html'>Page 1</a></li>
   <li><a href='/page_2.html'>Page 2</a></li>
   <li><a href='/page_3.html'>Page 3</a></li>
  </ul>
  <figure>
   <img src='img.jpg' alt='Awesome image'>
   <figcaption>Awesome image</figcaption>
  </figure>
  <footer>
   Last modified <time>2021-04-12</time>
  </footer>
  <!-- Thanks for reading -->
 </body>
</html>
```

*/

mod html;
pub use html::*;

use std::borrow::Cow;
use std::fmt::Write;
use std::sync::{Arc, Mutex, Weak};

/// A buffer for writing HTML into.
pub struct Buffer {
    ctx: Arc<Mutex<Ctx>>,
    node: Node<'static>,
}

/// An HTML element.
///
/// An open-tag is written to the buffer when a `Node` is created, and a
/// close-tag is written when the `Node` is dropped.  You can set attributes
/// on a newly-created node using [`attr()`][Node::attr], create child nodes
/// with [`child()`][Node::child], and write text into the node using the
/// `Write` impl.
///
/// ## Escaping
///
/// Text written into a node using its `Write` impl is transformed to make it
/// render correctly in an HTML-context.  By default, the following characters
/// are escaped: `&`, `<`, and `>`.  The escaping can be strengthened or
/// weakened using the [`safe()`][Node::safe] and [`raw()`][Node::raw]
/// methods respectively.
pub struct Node<'a> {
    depth: usize,
    ctx: Weak<Mutex<Ctx>>,
    escaping: Escaping,
    _phantom: std::marker::PhantomData<&'a ()>,
}

enum Escaping {
    Raw,
    Normal,
    Safe,
}

/// A self-closing element.
///
/// Void elements can't have any contents (since there's no end tag, no
/// content can be put between the start tag and the end tag).
pub struct Void<'a> {
    ctx: Weak<Mutex<Ctx>>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

/// A comment.
///
/// No escaping is performed on the contents.
pub struct Comment<'a> {
    ctx: Weak<Mutex<Ctx>>,
    _phantom: std::marker::PhantomData<&'a ()>,
}

#[derive(Default)]
struct Ctx {
    wtr: String,
    stack: Vec<(Cow<'static, str>, bool)>,
    tag_open: Option<&'static str>,
}

impl Buffer {
    /// Creates a new empty buffer.
    pub fn new() -> Buffer {
        Buffer::default()
    }

    /// Closes all open tags and returns the buffer's contents.
    pub fn finish(self) -> String {
        let mutex = Arc::try_unwrap(self.ctx).ok().unwrap();
        let mut ctx = mutex.into_inner().unwrap();
        ctx.close_deeper_than(0);
        ctx.wtr
    }
}

impl Default for Buffer {
    fn default() -> Buffer {
        let ctx = Arc::new(Mutex::new(Ctx::default()));
        let node = Node {
            depth: 0,
            ctx: Arc::downgrade(&ctx),
            escaping: Escaping::Normal,
            _phantom: std::marker::PhantomData,
        };
        Buffer { node, ctx }
    }
}

impl std::ops::Deref for Buffer {
    type Target = Node<'static>;
    fn deref(&self) -> &Node<'static> {
        &self.node
    }
}

impl std::ops::DerefMut for Buffer {
    fn deref_mut(&mut self) -> &mut Node<'static> {
        &mut self.node
    }
}

impl Ctx {
    fn close_unclosed(&mut self) {
        if let Some(closer) = self.tag_open.take() {
            self.wtr.write_str(closer).unwrap();
        }
    }

    fn close_deeper_than(&mut self, depth: usize) {
        self.close_unclosed();
        let to_pop = self.stack.len() - depth;
        for _ in 0..to_pop {
            if let Some((tag, is_self_closing)) = self.stack.pop() {
                if !is_self_closing {
                    write!(self.wtr, "</{}>", tag).unwrap();
                }
            }
        }
    }

    fn open(&mut self, tag: Cow<'static, str>, depth: usize, is_self_closing: bool) {
        self.close_deeper_than(depth);
        write!(self.wtr, "<{}", &tag).unwrap();
        if is_self_closing {
            self.tag_open = Some(" />");
        } else {
            self.tag_open = Some(">");
        }
        self.stack.push((tag, is_self_closing));
    }

    fn open_comment(&mut self, depth: usize) {
        self.close_deeper_than(depth);
        write!(self.wtr, "<!-- ").unwrap();
        self.tag_open = Some(" -->");
    }
}

impl<'a> Node<'a> {
    /// Create a new node, inheriting from the parent node.
    pub fn child<'b>(&'b mut self, tag: Cow<'static, str>) -> Node<'b> {
        let ctx = self.ctx.upgrade().unwrap();
        let mut ctx = ctx.lock().unwrap();
        ctx.open(tag, self.depth, false);
        Node {
            depth: self.depth + 1,
            ctx: self.ctx.clone(),
            escaping: Escaping::Normal,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Create a void child node, which self-closes.
    pub fn void_child<'b>(&'b mut self, tag: Cow<'static, str>) -> Void<'b> {
        let ctx = self.ctx.upgrade().unwrap();
        let mut ctx = ctx.lock().unwrap();
        ctx.open(tag, self.depth, true);
        Void {
            ctx: self.ctx.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn comment<'b>(&'b mut self) -> Comment<'b> {
        let ctx = self.ctx.upgrade().unwrap();
        let mut ctx = ctx.lock().unwrap();
        ctx.open_comment(self.depth);
        Comment {
            ctx: self.ctx.clone(),
            _phantom: std::marker::PhantomData,
        }
    }

    pub fn attr(self, attr: &str) -> Node<'a> {
        let ctx = self.ctx.upgrade().unwrap();
        let mut ctx = ctx.lock().unwrap();
        if ctx.tag_open.is_some() {
            write!(ctx.wtr, " {}", attr).unwrap();
        }
        self
    }

    /// Disable escaping
    ///
    /// In this mode, written text is passed through unmodified.
    pub fn raw(mut self) -> Node<'a> {
        self.escaping = Escaping::Raw;
        self
    }

    /// Escape more special characters
    ///
    /// In this mode, the following characters are escaped: `&`, `<`, `>`,
    /// `"`, `'`, and `/`.
    pub fn safe(mut self) -> Node<'a> {
        self.escaping = Escaping::Safe;
        self
    }
}

impl<'a> Write for Node<'a> {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let mutex = self.ctx.upgrade().unwrap();
        let mut ctx = mutex.lock().unwrap();
        ctx.close_deeper_than(self.depth);
        let s = match self.escaping {
            Escaping::Raw => s.into(),
            Escaping::Normal => html_escape::encode_text(s),
            Escaping::Safe => html_escape::encode_safe(s),
        };
        ctx.wtr.write_str(&s)
    }
}

impl<'a> Void<'a> {
    pub fn attr(self, attr: &str) -> Void<'a> {
        let ctx = self.ctx.upgrade().unwrap();
        let mut ctx = ctx.lock().unwrap();
        if ctx.tag_open.is_some() {
            write!(ctx.wtr, " {}", attr).unwrap();
        }
        self
    }
}

impl<'a> Write for Comment<'a> {
    fn write_char(&mut self, c: char) -> std::fmt::Result {
        let mutex = self.ctx.upgrade().unwrap();
        let mut ctx = mutex.lock().unwrap();
        ctx.wtr.write_char(c)
    }
    fn write_fmt(&mut self, args: std::fmt::Arguments<'_>) -> std::fmt::Result {
        let mutex = self.ctx.upgrade().unwrap();
        let mut ctx = mutex.lock().unwrap();
        ctx.wtr.write_fmt(args)
    }
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        let mutex = self.ctx.upgrade().unwrap();
        let mut ctx = mutex.lock().unwrap();
        ctx.wtr.write_str(s)
    }
}
