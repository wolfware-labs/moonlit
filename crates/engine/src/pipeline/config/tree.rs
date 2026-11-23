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

/// One parser event: a node, or the framing around nodes.
enum Tok {
  StreamStart,
  StreamEnd,
  DocStart,
  DocEnd,
  SeqEnd,
  MapEnd,
  Node(NodeTok),
}

/// An event that starts a node.
enum NodeTok {
  Scalar { raw: String, plain: bool, anchor: usize },
  SeqStart(usize),
  MapStart(usize),
  Alias(usize),
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
  // the limit as soon as it does, instead of after the whole event stream is in memory. Only the
  // first document is built, so later ones are read for syntax errors but not checked for depth.
  let mut open: usize = 0;
  let mut in_first_document = true;
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
      Event::Alias(id) => Tok::Node(NodeTok::Alias(id)),
      Event::Scalar(raw, style, anchor, _tag) => Tok::Node(NodeTok::Scalar {
        raw: raw.into_owned(),
        plain: matches!(style, ScalarStyle::Plain),
        anchor,
      }),
      Event::SequenceStart(anchor, _tag) => Tok::Node(NodeTok::SeqStart(anchor)),
      Event::SequenceEnd => Tok::SeqEnd,
      Event::MappingStart(anchor, _tag) => Tok::Node(NodeTok::MapStart(anchor)),
      Event::MappingEnd => Tok::MapEnd,
    };
    match tok {
      Tok::Node(NodeTok::SeqStart(_) | NodeTok::MapStart(_)) if in_first_document => {
        check_depth(src, open, 1, span)?;
        open += 1;
      }
      Tok::SeqEnd | Tok::MapEnd => open = open.saturating_sub(1),
      Tok::DocEnd => in_first_document = false,
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
  /// What the aliases read so far expand to in the document.
  aliases: AliasBudget,
  /// The release file, for diagnostics.
  src: &'t ConfigSource<'a>,
}

impl<'t> Builder<'t, '_> {
  fn build_document(&mut self) -> Result<Node, ConfigDiagnostic> {
    loop {
      match self.toks.get(self.pos) {
        Some((Tok::StreamStart | Tok::DocStart, _)) => self.pos += 1,
        // An anchored root is still shared with `anchors`; copying it then is shallow.
        Some((Tok::Node(tok), span)) => return self.build_node(tok, *span, 0).map(|(root, _)| Rc::unwrap_or_clone(root)),
        // The document or the stream ends before any node: an empty document.
        Some((_, s)) => return Ok(Node::null(*s)),
        None => return Ok(Node::null(Span::point(0))),
      }
    }
  }

  /// Builds the node that `tok` starts at the cursor, which sits under `enclosing` collections, and
  /// returns it with its expanded size. Refuses an alias that would nest past [`MAX_DEPTH`] or take
  /// the document past either alias limit; [`build_tree`] has already refused input nested past
  /// [`MAX_DEPTH`].
  fn build_node(&mut self, tok: &'t NodeTok, span: Span, enclosing: usize) -> Result<(Rc<Node>, Size), ConfigDiagnostic> {
    self.pos += 1;
    match tok {
      NodeTok::Scalar { raw, plain, anchor } => {
        let size = Size::scalar(raw.len());
        let value = if *plain && is_yaml_null(raw) {
          NodeValue::Null
        } else {
          NodeValue::Scalar(raw.clone())
        };
        let node = Rc::new(Node { value, span });
        self.register(*anchor, &node, size);
        Ok((node, size))
      }
      NodeTok::SeqStart(anchor) => {
        let (items, size, end) = self.build_children(span, enclosing)?;
        let node = Rc::new(Node {
          value: NodeValue::Seq(items),
          span: Span::new(span.start, end.end),
        });
        self.register(*anchor, &node, size);
        Ok((node, size))
      }
      NodeTok::MapStart(anchor) => {
        // A mapping's children alternate between keys and values. The parser emits an empty scalar
        // for a key or value that is not written, so the count is even and no key is dropped.
        let (children, size, end) = self.build_children(span, enclosing)?;
        let mut children = children.into_iter();
        let mut entries = Vec::new();
        while let (Some(key), Some(val)) = (children.next(), children.next()) {
          entries.push((key, val));
        }
        let node = Rc::new(Node {
          value: NodeValue::Map(entries),
          span: Span::new(span.start, end.end),
        });
        self.register(*anchor, &node, size);
        Ok((node, size))
      }
      NodeTok::Alias(id) => match self.anchors.get(id) {
        Some((anchored, size)) => {
          check_depth(self.src, enclosing, size.height, span)?;
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
    }
  }

  /// Builds the children of the collection opened at `start` under `enclosing` others, up to and
  /// including its closing event. Returns them with the collection's expanded size and the span of
  /// that closing event.
  fn build_children(&mut self, start: Span, enclosing: usize) -> Result<(Vec<Rc<Node>>, Size, Span), ConfigDiagnostic> {
    let level = enclosing + 1; // the reader refused anything deeper than MAX_DEPTH
    let mut size = Size::empty_collection();
    let mut children = Vec::new();
    loop {
      match self.toks.get(self.pos) {
        Some((Tok::Node(tok), span)) => {
          let (child, child_size) = self.build_node(tok, *span, level)?;
          size.add_child(child_size);
          children.push(child);
        }
        // The parser's events are balanced, so the first event that does not start a node closes
        // this collection.
        other => {
          self.pos += 1;
          return Ok((children, size, other.map_or(start, |(_, s)| *s)));
        }
      }
    }
  }

  fn register(&mut self, anchor: usize, node: &Rc<Node>, size: Size) {
    if anchor != 0 {
      self.anchors.insert(anchor, (Rc::clone(node), size));
    }
  }
}

/// Refuses a subtree of `height` levels placed under `enclosing` collections when it nests past
/// [`MAX_DEPTH`], with the diagnostic at `span`.
fn check_depth(src: &ConfigSource, enclosing: usize, height: usize, span: Span) -> Result<(), ConfigDiagnostic> {
  if limits::too_deep(enclosing, height) {
    return Err(ConfigDiagnostic::nesting_too_deep(src, MAX_DEPTH, span));
  }
  Ok(())
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

  /// Whether `a` and `b` are sequences of two items holding the very same child nodes.
  fn share_two_children(a: &Node, b: &Node) -> bool {
    matches!(
      (&a.value, &b.value),
      (NodeValue::Seq(x), NodeValue::Seq(y)) if x.len() == 2 && x.iter().zip(y).all(|(p, q)| Rc::ptr_eq(p, q))
    )
  }

  #[test]
  fn alias_shares_the_anchored_children() {
    let tree = build_tree(&ConfigSource::new("a: &x [one, two]\nb: *x\n", "release.yml")).expect("valid tree");
    // The alias node itself carries the alias site's span.
    assert!(matches!(
      &tree.value,
      NodeValue::Map(entries) if share_two_children(&entries[0].1, &entries[1].1) && entries[1].1.span == Span::new(20, 22)
    ));
  }

  #[test]
  fn nested_anchor_chain_just_under_the_alias_limits_builds() {
    let yaml = nested_anchor_chain();
    let tree = build_tree(&ConfigSource::new(&yaml, "release.yml")).expect("within both alias limits");
    assert!(matches!(&tree.value, NodeValue::Map(entries) if entries.len() == 3));
  }
}
