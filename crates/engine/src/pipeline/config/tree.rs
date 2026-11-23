use crate::pipeline::config::diagnostic::ConfigDiagnostic;
use crate::pipeline::config::limits::{self, AliasBudget, MAX_ALIAS_BYTES, MAX_ALIAS_NODES, MAX_DEPTH, Size};
use crate::pipeline::config::source::ConfigSource;
use crate::pipeline::config::span::Span;
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
  #[must_use]
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
  Scalar { raw: String, plain: bool, anchor: usize },
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

#[must_use]
fn is_yaml_null(raw: &str) -> bool {
  matches!(raw, "" | "~" | "null" | "Null" | "NULL")
}

pub fn build_tree(src: &ConfigSource) -> Result<Node, ConfigDiagnostic> {
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
      ConfigDiagnostic::invalid_syntax(src, e.info(), Span::point(byte))
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
    aliases: AliasBudget::default(),
    src,
  }
  .build_document()
}

struct Builder<'t, 'a> {
  toks: &'t [(Tok, Span)],
  pos: usize,
  /// Each anchored node with its expanded size, which every alias to it is charged.
  anchors: HashMap<usize, (Node, Size)>,
  aliases: AliasBudget,
  src: &'t ConfigSource<'a>,
}

impl Builder<'_, '_> {
  #[must_use]
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
        Some(_) => return self.build_node(0).map(|(node, _)| node),
        None => return Ok(Node::null(Span::point(0))),
      }
    }
  }

  /// Builds the node at the cursor, which sits under `enclosing` collections, and returns it with
  /// its expanded size. Refuses a collection past [`MAX_DEPTH`] before descending into it, and an
  /// alias past either limit before copying its anchor.
  fn build_node(&mut self, enclosing: usize) -> Result<(Node, Size), ConfigDiagnostic> {
    let idx = self.pos;
    self.pos += 1;
    let (tok, span) = match self.toks.get(idx) {
      Some((t, s)) => (t.clone(), *s),
      None => return Ok((Node::null(Span::point(0)), Size::scalar(0))),
    };
    match tok {
      Tok::Scalar { raw, plain, anchor } => {
        let size = Size::scalar(raw.len());
        let value = if plain && is_yaml_null(&raw) {
          NodeValue::Null
        } else {
          NodeValue::Scalar(raw)
        };
        let node = Node { value, span };
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::SeqStart(anchor) => {
        let level = self.enter(enclosing, span)?;
        let mut size = Size::empty_collection();
        let mut items = Vec::new();
        let end = loop {
          match self.peek(EndKind::Seq) {
            Peek::End(s) => {
              self.pos += 1;
              break s;
            }
            Peek::Item => {
              let (item, item_size) = self.build_node(level)?;
              size.add_child(item_size);
              items.push(item);
            }
            Peek::Eof => break span,
          }
        };
        let node = Node {
          value: NodeValue::Seq(items),
          span: Span::new(span.start, end.end),
        };
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::MapStart(anchor) => {
        let level = self.enter(enclosing, span)?;
        let mut size = Size::empty_collection();
        let mut entries = Vec::new();
        let end = loop {
          match self.peek(EndKind::Map) {
            Peek::End(s) => {
              self.pos += 1;
              break s;
            }
            Peek::Item => {
              let (key, key_size) = self.build_node(level)?;
              let (val, val_size) = self.build_node(level)?;
              size.add_child(key_size);
              size.add_child(val_size);
              entries.push((key, val));
            }
            Peek::Eof => break span,
          }
        };
        let node = Node {
          value: NodeValue::Map(entries),
          span: Span::new(span.start, end.end),
        };
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::Alias(id) => match self.anchors.get(&id) {
        Some((anchored, size)) => {
          if limits::too_deep(enclosing, size.height) {
            return Err(ConfigDiagnostic::nesting_too_deep(self.src, MAX_DEPTH, span));
          }
          if !self.aliases.charge(*size) {
            return Err(ConfigDiagnostic::alias_expansion_too_large(
              self.src,
              MAX_ALIAS_NODES,
              MAX_ALIAS_BYTES,
              span,
            ));
          }
          let mut node = anchored.clone();
          node.span = span; // carry the alias site's span, not the anchor's
          Ok((node, *size))
        }
        None => Err(ConfigDiagnostic::unknown_alias(self.src, span)),
      },
      // Framing tokens shouldn't reach here; treat defensively as empty.
      Tok::StreamStart | Tok::StreamEnd | Tok::DocStart | Tok::DocEnd | Tok::SeqEnd | Tok::MapEnd => {
        Ok((Node::null(span), Size::scalar(0)))
      }
    }
  }

  /// Opens a collection under `enclosing` others and returns its own level, or refuses it when that
  /// level passes [`MAX_DEPTH`].
  fn enter(&self, enclosing: usize, span: Span) -> Result<usize, ConfigDiagnostic> {
    if limits::too_deep(enclosing, 1) {
      return Err(ConfigDiagnostic::nesting_too_deep(self.src, MAX_DEPTH, span));
    }
    Ok(enclosing + 1)
  }

  fn register(&mut self, anchor: usize, node: &Node, size: Size) {
    if anchor != 0 {
      self.anchors.insert(anchor, (node.clone(), size));
    }
  }
}
