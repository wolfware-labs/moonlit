use crate::pipeline::config::diagnostic::{ConfigDiagnostic, Source};
use crate::pipeline::config::model::Span;
use saphyr_parser::{Event, Parser, ScalarStyle, Span as PSpan};
use std::collections::HashMap;

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

#[derive(Clone, Copy)]
enum Peek {
    Item,
    End(Span),
    Eof,
}

fn is_yaml_null(raw: &str) -> bool {
    matches!(raw, "" | "~" | "null" | "Null" | "NULL")
}

pub fn build_tree(src: &Source) -> Result<Node, ConfigDiagnostic> {
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
