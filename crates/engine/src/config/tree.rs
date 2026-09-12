//! YAML event stream → spanned node tree, with anchor/alias resolution.
//!
//! The only module that touches `saphyr-parser`. It produces a schema-agnostic tree; `convert`
//! maps that tree onto the typed model. Events are first lowered into owned [`Tok`]s so the tree
//! builder never holds a borrow of the parser output across a recursive call.

use std::collections::HashMap;

use saphyr_parser::{Event, Parser, ScalarStyle, Span as PSpan};

use crate::config::diagnostic::{ConfigDiagnostic, Source};
use crate::config::model::Span;

/// A schema-agnostic YAML node with its source span.
#[derive(Clone, Debug, PartialEq)]
pub struct Node {
    pub value: NodeValue,
    pub span: Span,
}

#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
    Null,
    Scalar(String),
    Seq(Vec<Node>),
    Map(Vec<(Node, Node)>),
}

impl Node {
    fn null(span: Span) -> Self {
        Node {
            value: NodeValue::Null,
            span,
        }
    }
}

/// An owned lowering of `saphyr_parser::Event`, decoupled from the parser's lifetime.
#[derive(Clone)]
enum Tok {
    StreamStart,
    StreamEnd,
    DocStart,
    DocEnd,
    Scalar {
        raw: String,
        plain: bool,
        anchor: usize,
    },
    SeqStart(usize),
    SeqEnd,
    MapStart(usize),
    MapEnd,
    Alias(usize),
}

#[derive(Clone, Copy, PartialEq)]
enum EndKind {
    Seq,
    Map,
}

/// Result of peeking at the next token without consuming it.
#[derive(Clone, Copy)]
enum Peek {
    Item,
    End(Span),
    Eof,
}

fn is_yaml_null(raw: &str) -> bool {
    matches!(raw, "" | "~" | "null" | "Null" | "NULL")
}

/// Parse the first YAML document in `src.yaml` into a spanned node tree.
pub fn build_tree(src: &Source) -> Result<Node, ConfigDiagnostic> {
    // saphyr-parser 0.0.11's `Marker::index()` is a CHAR index — its accessor doc
    // claiming "bytes" is wrong (the field doc says chars, and the scanner advances the
    // index per char). miette source spans are BYTE offsets, so map each char index to a
    // byte offset via the source. `char_to_byte[ci]` is the byte offset of the ci-th char;
    // the trailing sentinel maps the one-past-the-end index to the byte length.
    let char_to_byte: Vec<usize> = {
        let mut v: Vec<usize> = src.yaml.char_indices().map(|(b, _)| b).collect();
        v.push(src.yaml.len());
        v
    };
    let to_span = |p: PSpan| {
        let len = src.yaml.len();
        Span::new(
            char_to_byte.get(p.start.index()).copied().unwrap_or(len),
            char_to_byte.get(p.end.index()).copied().unwrap_or(len),
        )
    };

    let mut toks: Vec<(Tok, Span)> = Vec::new();
    for item in Parser::new_from_str(src.yaml) {
        let (ev, pspan) = item.map_err(|e| {
            let len = src.yaml.len();
            let byte = char_to_byte.get(e.marker().index()).copied().unwrap_or(len);
            src.syntax(e.info(), Span::point(byte))
        })?;
        let span = to_span(pspan);
        let tok = match ev {
            Event::Nothing => continue,
            Event::StreamStart => Tok::StreamStart,
            Event::StreamEnd => Tok::StreamEnd,
            Event::DocumentStart(_) => Tok::DocStart,
            Event::DocumentEnd => Tok::DocEnd,
            Event::Alias(id) => Tok::Alias(id),
            Event::Scalar(raw, style, anchor, _tag) => Tok::Scalar {
                raw: raw.into_owned(),
                plain: matches!(style, ScalarStyle::Plain),
                anchor,
            },
            Event::SequenceStart(anchor, _tag) => Tok::SeqStart(anchor),
            Event::SequenceEnd => Tok::SeqEnd,
            Event::MappingStart(anchor, _tag) => Tok::MapStart(anchor),
            Event::MappingEnd => Tok::MapEnd,
        };
        toks.push((tok, span));
    }

    Builder {
        toks: &toks,
        pos: 0,
        anchors: HashMap::new(),
        src,
    }
    .build_document()
}

struct Builder<'t, 'a> {
    toks: &'t [(Tok, Span)],
    pos: usize,
    anchors: HashMap<usize, Node>,
    src: &'t Source<'a>,
}

impl<'t, 'a> Builder<'t, 'a> {
    fn peek(&self, want: EndKind) -> Peek {
        match self.toks.get(self.pos) {
            None => Peek::Eof,
            Some((Tok::SeqEnd, s)) if want == EndKind::Seq => Peek::End(*s),
            Some((Tok::MapEnd, s)) if want == EndKind::Map => Peek::End(*s),
            Some(_) => Peek::Item,
        }
    }

    fn build_document(&mut self) -> Result<Node, ConfigDiagnostic> {
        loop {
            match self.toks.get(self.pos) {
                Some((Tok::StreamStart | Tok::DocStart, _)) => self.pos += 1,
                Some((Tok::StreamEnd | Tok::DocEnd, s)) => return Ok(Node::null(*s)),
                Some(_) => return self.build_node(),
                None => return Ok(Node::null(Span::point(0))),
            }
        }
    }

    fn build_node(&mut self) -> Result<Node, ConfigDiagnostic> {
        let idx = self.pos;
        self.pos += 1;
        let (tok, span) = match self.toks.get(idx) {
            Some((t, s)) => (t.clone(), *s),
            None => return Ok(Node::null(Span::point(0))),
        };
        match tok {
            Tok::Scalar { raw, plain, anchor } => {
                let value = if plain && is_yaml_null(&raw) {
                    NodeValue::Null
                } else {
                    NodeValue::Scalar(raw)
                };
                let node = Node { value, span };
                self.register(anchor, &node);
                Ok(node)
            }
            Tok::SeqStart(anchor) => {
                let mut items = Vec::new();
                let end = loop {
                    match self.peek(EndKind::Seq) {
                        Peek::End(s) => {
                            self.pos += 1;
                            break s;
                        }
                        Peek::Item => items.push(self.build_node()?),
                        Peek::Eof => break span,
                    }
                };
                let node = Node {
                    value: NodeValue::Seq(items),
                    span: Span::new(span.start, end.end),
                };
                self.register(anchor, &node);
                Ok(node)
            }
            Tok::MapStart(anchor) => {
                let mut entries = Vec::new();
                let end = loop {
                    match self.peek(EndKind::Map) {
                        Peek::End(s) => {
                            self.pos += 1;
                            break s;
                        }
                        Peek::Item => {
                            let key = self.build_node()?;
                            let val = self.build_node()?;
                            entries.push((key, val));
                        }
                        Peek::Eof => break span,
                    }
                };
                let node = Node {
                    value: NodeValue::Map(entries),
                    span: Span::new(span.start, end.end),
                };
                self.register(anchor, &node);
                Ok(node)
            }
            Tok::Alias(id) => match self.anchors.get(&id) {
                Some(anchored) => {
                    let mut node = anchored.clone();
                    node.span = span; // carry the alias site's span, not the anchor's
                    Ok(node)
                }
                None => Err(self.src.unknown_alias(span)),
            },
            // Framing tokens shouldn't reach here; treat defensively as empty.
            Tok::StreamStart
            | Tok::StreamEnd
            | Tok::DocStart
            | Tok::DocEnd
            | Tok::SeqEnd
            | Tok::MapEnd => Ok(Node::null(span)),
        }
    }

    fn register(&mut self, anchor: usize, node: &Node) {
        if anchor != 0 {
            self.anchors.insert(anchor, node.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::diagnostic::Source;

    fn tree(yaml: &str) -> Node {
        build_tree(&Source::new(yaml, "release.yml")).expect("valid yaml")
    }

    fn map(n: &Node) -> &[(Node, Node)] {
        match &n.value {
            NodeValue::Map(e) => e,
            other => panic!("expected map, got {other:?}"),
        }
    }

    #[test]
    fn scalars_sequences_and_maps() {
        let root = tree("name: demo\nitems:\n  - a\n  - b\n");
        let entries = map(&root);
        assert_eq!(entries.len(), 2);
        assert!(matches!(&entries[0].0.value, NodeValue::Scalar(k) if k == "name"));
        assert!(matches!(&entries[0].1.value, NodeValue::Scalar(v) if v == "demo"));
        match &entries[1].1.value {
            NodeValue::Seq(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(&items[0].value, NodeValue::Scalar(s) if s == "a"));
            }
            other => panic!("expected seq, got {other:?}"),
        }
    }

    #[test]
    fn plain_null_vs_quoted_empty() {
        let root = tree("a:\nb: \"\"\nc: ~\nd: null\n");
        let e = map(&root);
        assert!(matches!(e[0].1.value, NodeValue::Null), "bare -> null");
        assert!(
            matches!(&e[1].1.value, NodeValue::Scalar(s) if s.is_empty()),
            "quoted -> empty string"
        );
        assert!(matches!(e[2].1.value, NodeValue::Null), "~ -> null");
        assert!(matches!(e[3].1.value, NodeValue::Null), "null -> null");
    }

    #[test]
    fn byte_spans_not_char_spans() {
        // "café" is 5 bytes (é is 2). The key node span must index bytes.
        let yaml = "café: 1\n";
        let root = tree(yaml);
        let key = &map(&root)[0].0;
        assert_eq!(&yaml[key.span.start..key.span.end], "café");
    }

    #[test]
    fn alias_resolves_to_anchored_subtree_with_alias_site_span() {
        let yaml = "defs:\n  base: &b {x: 1}\nuse: *b\n";
        let root = tree(yaml);
        let e = map(&root);
        // `use: *b` — the value is a clone of {x: 1}...
        let used = &e[1].1;
        match &used.value {
            NodeValue::Map(inner) => {
                assert!(matches!(&inner[0].0.value, NodeValue::Scalar(k) if k == "x"));
            }
            other => panic!("expected aliased map, got {other:?}"),
        }
        // ...but its span points at the alias site `*b`, not the anchor definition.
        let alias_at = yaml.find("*b").unwrap();
        assert!(
            used.span.start >= alias_at,
            "alias node carries the alias-site span"
        );
    }

    #[test]
    fn unknown_alias_is_an_error() {
        // saphyr-parser 0.0.11 rejects an undefined alias during scanning (before an
        // Alias event is emitted), so it surfaces as a syntax diagnostic. The Builder's
        // own `unknown_alias` guard remains for the (not-reached-by-this-parser) case
        // where a parse otherwise succeeds with a dangling alias id.
        let err = build_tree(&Source::new("use: *missing\n", "release.yml")).unwrap_err();
        let msg = err.message();
        assert!(
            msg.contains("anchor") || msg.contains("alias"),
            "expected an unknown-alias/anchor error, got: {msg}"
        );
    }

    #[test]
    fn malformed_yaml_is_a_diagnostic() {
        let err = build_tree(&Source::new("a: [1, 2\n", "release.yml")).unwrap_err();
        assert!(err.message().contains("Invalid YAML"));
    }

    #[test]
    fn empty_document_is_null_root() {
        let root = build_tree(&Source::new("# just a comment\n", "release.yml")).unwrap();
        assert!(matches!(root.value, NodeValue::Null));
    }
}
