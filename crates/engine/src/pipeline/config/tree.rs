use crate::pipeline::config::diagnostic::ConfigDiagnostic;
use crate::pipeline::config::limits::{self, AliasBudget, MAX_ALIAS_BYTES, MAX_ALIAS_NODES, MAX_DEPTH, Size};
use crate::pipeline::config::source::ConfigSource;
use crate::pipeline::config::span::Span;
use saphyr_parser::{Event, Parser, ScalarStyle, Span as PSpan};
use std::collections::HashMap;
use std::rc::Rc;

#[derive(Clone, Debug, PartialEq)]
pub struct Node {
  pub value: NodeValue,
  pub span: Span,
}

/// A node's value. Children are shared, so an anchored node is stored once however many aliases
/// name it: an alias copies only the node it names, not the subtree below it.
#[derive(Clone, Debug, PartialEq)]
pub enum NodeValue {
  Null,
  Scalar(String),
  Seq(Vec<Rc<Node>>),
  Map(Vec<(Rc<Node>, Rc<Node>)>),
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
  // Collections open at the current event. Counting them here stops reading a file that nests past
  // the limit as soon as it does, instead of after the whole event stream is in memory.
  let mut open: usize = 0;
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
    match tok {
      Tok::SeqStart(_) | Tok::MapStart(_) => {
        if limits::too_deep(open, 1) {
          return Err(ConfigDiagnostic::nesting_too_deep(src, MAX_DEPTH, span));
        }
        open += 1;
      }
      Tok::SeqEnd | Tok::MapEnd => open = open.saturating_sub(1),
      _ => {}
    }
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
  /// The document's events, with their byte spans.
  toks: &'t [(Tok, Span)],
  /// Index of the next token to read.
  pos: usize,
  /// Each anchored node, shared with the tree, and its expanded size, which every alias to it is
  /// charged.
  anchors: HashMap<usize, (Rc<Node>, Size)>,
  /// What the aliases read so far have copied into the document.
  aliases: AliasBudget,
  /// The release file, for diagnostics.
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
        // An anchored root is still shared with `anchors`; copying it then is shallow.
        Some(_) => return self.build_node(0).map(|(root, _)| Rc::unwrap_or_clone(root)),
        None => return Ok(Node::null(Span::point(0))),
      }
    }
  }

  /// Builds the node at the cursor, which sits under `enclosing` collections, and returns it with
  /// its expanded size. Refuses an alias that would nest past [`MAX_DEPTH`] or take the document
  /// past either alias limit; [`build_tree`] has already refused input nested past [`MAX_DEPTH`].
  fn build_node(&mut self, enclosing: usize) -> Result<(Rc<Node>, Size), ConfigDiagnostic> {
    let idx = self.pos;
    self.pos += 1;
    let (tok, span) = match self.toks.get(idx) {
      Some((t, s)) => (t.clone(), *s),
      None => return Ok((Rc::new(Node::null(Span::point(0))), Size::scalar(0))),
    };
    match tok {
      Tok::Scalar { raw, plain, anchor } => {
        let size = Size::scalar(raw.len());
        let value = if plain && is_yaml_null(&raw) {
          NodeValue::Null
        } else {
          NodeValue::Scalar(raw)
        };
        let node = Rc::new(Node { value, span });
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::SeqStart(anchor) => {
        let level = enclosing + 1; // the reader refused anything deeper than MAX_DEPTH
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
        let node = Rc::new(Node {
          value: NodeValue::Seq(items),
          span: Span::new(span.start, end.end),
        });
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::MapStart(anchor) => {
        let level = enclosing + 1; // the reader refused anything deeper than MAX_DEPTH
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
        let node = Rc::new(Node {
          value: NodeValue::Map(entries),
          span: Span::new(span.start, end.end),
        });
        self.register(anchor, &node, size);
        Ok((node, size))
      }
      Tok::Alias(id) => match self.anchors.get(&id) {
        Some((anchored, size)) => {
          self.check_depth(enclosing, size.height, span)?;
          if !self.aliases.charge(*size) {
            return Err(ConfigDiagnostic::alias_expansion_too_large(
              self.src,
              MAX_ALIAS_NODES,
              MAX_ALIAS_BYTES,
              span,
            ));
          }
          // Copy the named node only, to carry the alias site's span; its children stay shared.
          let node = Node {
            value: anchored.value.clone(),
            span,
          };
          Ok((Rc::new(node), *size))
        }
        None => Err(ConfigDiagnostic::unknown_alias(self.src, span)),
      },
      // Framing tokens shouldn't reach here; treat defensively as empty.
      Tok::StreamStart | Tok::StreamEnd | Tok::DocStart | Tok::DocEnd | Tok::SeqEnd | Tok::MapEnd => {
        Ok((Rc::new(Node::null(span)), Size::scalar(0)))
      }
    }
  }

  /// Refuses a subtree of `height` levels placed under `enclosing` collections when it nests past
  /// [`MAX_DEPTH`], with the diagnostic at `span`.
  fn check_depth(&self, enclosing: usize, height: usize, span: Span) -> Result<(), ConfigDiagnostic> {
    if limits::too_deep(enclosing, height) {
      return Err(ConfigDiagnostic::nesting_too_deep(self.src, MAX_DEPTH, span));
    }
    Ok(())
  }

  fn register(&mut self, anchor: usize, node: &Rc<Node>, size: Size) {
    if anchor != 0 {
      self.anchors.insert(anchor, (Rc::clone(node), size));
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  /// One 128 KiB scalar and a 1000-item list, then 62 anchored sequences nested in one another,
  /// the innermost listing 62 aliases of the scalar and 99 of the list: just under both alias
  /// limits, with every level anchored.
  fn nested_anchor_chain() -> String {
    let aliases = [vec!["*s"; 62], vec!["*n"; 99]].concat().join(", ");
    let open = (0..62).map(|i| format!("&a{i} [")).collect::<Vec<_>>().concat();
    format!(
      "s: &s \"{}\"\nn: &n [{}]\nc: {open}{aliases}{}\n",
      "x".repeat(128 << 10),
      vec!["x"; 1000].join(", "),
      "]".repeat(62)
    )
  }

  #[test]
  fn alias_shares_the_anchored_children() {
    let tree = build_tree(&ConfigSource::new("a: &x [one, two]\nb: *x\n", "release.yml")).expect("valid tree");
    let NodeValue::Map(entries) = &tree.value else {
      panic!("root is a mapping")
    };
    let (NodeValue::Seq(anchored), NodeValue::Seq(aliased)) = (&entries[0].1.value, &entries[1].1.value) else {
      panic!("both values are sequences")
    };
    assert!(anchored.iter().zip(aliased).all(|(a, b)| Rc::ptr_eq(a, b)));
    // The alias node itself carries the alias site's span.
    assert_eq!(entries[1].1.span, Span::new(20, 22));
  }

  #[test]
  fn nested_anchor_chain_stays_within_budget_memory() {
    let yaml = nested_anchor_chain();
    let tree = build_tree(&ConfigSource::new(&yaml, "release.yml")).expect("within both alias limits");
    let NodeValue::Map(entries) = &tree.value else {
      panic!("root is a mapping")
    };
    assert_eq!(entries.len(), 3);
  }
}
