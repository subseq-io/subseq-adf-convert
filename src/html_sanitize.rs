use std::rc::Rc;

use html5ever::serialize::{SerializeOpts, serialize};
use html5ever::tendril::Tendril;
use html5ever::{parse_document, tendril::TendrilSink};
use markup5ever_rcdom::{Handle, Node, NodeData, RcDom, SerializableHandle};
use std::default::Default;

pub fn sanitize_html_structure(input: &str) -> String {
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .unwrap();

    sanitize_node(&dom.document);

    let mut output = Vec::new();
    serialize(
        &mut output,
        &SerializableHandle::from(dom.document.clone()),
        SerializeOpts::default(),
    )
    .unwrap();

    String::from_utf8(output).unwrap()
}

fn sanitize_node(handle: &Handle) {
    let mut to_recurse = vec![];
    {
        let node = handle;
        if let NodeData::Element { ref name, .. } = node.data {
            if name.local.as_ref() == "a" {
                unwrap_nested_anchors(node);
            } else if name.local.as_ref() == "p" {
                let mut has_block = false;
                for child in node.children.borrow().iter() {
                    if is_known_block_element(child) {
                        has_block = true;
                        break;
                    }
                }

                if has_block {
                    if let Some(parent_weak) = node.parent.take() {
                        if let Some(parent) = parent_weak.upgrade() {
                            let mut parent_children = parent.children.borrow_mut();

                            if let Some(index) =
                                parent_children.iter().position(|n| Rc::ptr_eq(n, node))
                            {
                                parent_children.remove(index);
                                for child in node.children.borrow().iter() {
                                    parent_children.insert(index, child.clone());
                                }
                            }

                            // Restore parent back after mutation
                            node.parent.set(Some(Rc::downgrade(&parent)));
                        } else {
                            // Parent weak reference was dangling; restore as None
                            node.parent.set(None);
                        }
                    }
                }
            }
        }

        // Recurse into children
        to_recurse.extend(node.children.borrow().iter().cloned());
    }

    for child in to_recurse {
        sanitize_node(&child);
    }
}

fn make_text_node(text: &str) -> Handle {
    Rc::new(Node {
        parent: Default::default(),
        children: Default::default(),
        data: NodeData::Text {
            contents: std::cell::RefCell::new(Tendril::from(text.to_string())),
        },
    })
}

fn unwrap_nested_anchors(node: &Handle) {
    let mut children = node.children.borrow_mut();
    let mut i = 0;
    while i < children.len() {
        if let NodeData::Element { name, .. } = &children[i].data {
            if name.local.as_ref() == "a" {
                // Extract text nodes from nested <a>
                let text_nodes: Vec<_> = children[i]
                    .children
                    .borrow()
                    .iter()
                    .filter_map(|child| match &child.data {
                        NodeData::Text { contents } => Some(contents.borrow().clone()),
                        _ => None,
                    })
                    .collect();

                let inserts: Vec<_> = text_nodes
                    .into_iter()
                    .map(|t| make_text_node(t.as_ref()))
                    .collect();

                children.remove(i);
                children.splice(i..i, inserts);
                continue; // stay at the same index
            }
        }
        i += 1;
    }
}

fn is_known_block_element(node: &Handle) -> bool {
    if let NodeData::Element { ref name, .. } = node.data {
        match name.local.as_ref() {
            "details" | "summary" | "adf-media-group" | "table" => true,
            _ => false,
        }
    } else {
        false
    }
}

pub fn normalize_html(input: &str) -> String {
    let dom = parse_document(RcDom::default(), Default::default())
        .from_utf8()
        .read_from(&mut input.as_bytes())
        .unwrap();

    let mut output = Vec::new();
    serialize(
        &mut output,
        &markup5ever_rcdom::SerializableHandle::from(dom.document.clone()),
        SerializeOpts::default(),
    )
    .unwrap();

    String::from_utf8(output).unwrap()
}

#[cfg(test)]
mod tests {
    use super::normalize_html;
    use super::sanitize_html_structure;

    #[test]
    fn test_sanitize_unclosed_p_with_block() {
        let raw_html = r#"
            <body>
                <p><details><summary>More info</summary>
                <p>Extra details.</p>
                </details>
            </body>
        "#;

        let sanitized = sanitize_html_structure(raw_html);

        // Ensure no <details> is ever inside a <p>
        assert!(
            !sanitized.contains("<p><details>"),
            "Sanitized HTML must not contain <details> inside <p>"
        );

        // Ensure the <details> block still exists and is outside of any <p>
        assert!(
            sanitized.contains("<details>"),
            "Sanitized HTML must retain <details> block"
        );

        // Ensure the <p> blocks are closed and not improperly nested
        let normalized = normalize_html(&sanitized);
        assert!(
            normalized.contains("<details>"),
            "Re-parsed HTML must contain <details>"
        );
        assert!(
            normalized.contains("<p>"),
            "Re-parsed HTML must contain paragraphs"
        );
    }
}
