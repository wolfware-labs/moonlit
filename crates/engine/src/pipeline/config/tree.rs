use crate::pipeline::config::diagnostic::ConfigDiagnostic;
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

#[must_use]
fn quoted_end(yaml: &str, span: Span) -> usize {
  let text = &yaml[span.start..span.end];
  let mut chars = text.char_indices();
  let Some((_, quote @ ('\'' | '"'))) = chars.next() else {
    return span.end;
  };
  while let Some((i, c)) = chars.next() {
    if quote == '"' && c == '\\' {
      chars.next();
    } else if c == quote {
      if quote == '\'' && text[i + 1..].starts_with('\'') {
        chars.next();
      } else {
        return span.start + i + 1;
      }
    }
  }
  span.end
}

#[must_use]
fn block_end(yaml: &str, span: Span, raw: &str) -> usize {
  let raw_content = raw.trim_end().len();
  if raw_content == 0 {
    return span.end;
  }
  let content = span.start + yaml[span.start..span.end].trim_end().len();
  let kept = raw[raw_content..].matches('\n').count();
  if kept < 2 {
    return content;
  }
  let after = &yaml[content..];
  after[..after.len() - after.trim_start().len()]
    .match_indices('\n')
    .nth(kept - 1)
    .map_or(content, |(i, _)| content + i)
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
    let mut span = to_span(pspan);
    match &ev {
      Event::Scalar(_, ScalarStyle::SingleQuoted | ScalarStyle::DoubleQuoted, ..) => span.end = quoted_end(src.yaml, span),
      Event::Scalar(raw, ScalarStyle::Literal | ScalarStyle::Folded, ..) => span.end = block_end(src.yaml, span, raw),
      _ => {}
    }
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
          span: Span::new(span.start, self.collection_end(end)),
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
          span: Span::new(span.start, self.collection_end(end)),
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
        None => Err(ConfigDiagnostic::unknown_alias(self.src, span)),
      },
      // Framing tokens shouldn't reach here; treat defensively as empty.
      Tok::StreamStart | Tok::StreamEnd | Tok::DocStart | Tok::DocEnd | Tok::SeqEnd | Tok::MapEnd => Ok(Node::null(span)),
    }
  }

  #[must_use]
  fn collection_end(&self, end: Span) -> usize {
    if self.src.yaml[end.start..].starts_with([']', '}']) {
      end.start + 1
    } else {
      end.end
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

  fn value_text(yaml: &str) -> String {
    value_texts(yaml)
      .into_iter()
      .find_map(|(key, value)| (key == "k").then_some(value))
      .expect("key k")
  }

  #[test]
  fn a_double_quoted_scalar_spans_through_an_escaped_quote() {
    assert_eq!(value_text("k: \"a\\\"b\"\nz: 1\n"), "\"a\\\"b\"");
  }

  #[test]
  fn a_quoted_scalar_ends_at_its_closing_quote_before_a_comment() {
    assert_eq!(value_text("k: \"a\\\"b\"  # c\nz: 1\n"), "\"a\\\"b\"");
    assert_eq!(value_text("k: 'a\\'  # c\nz: 1\n"), "'a\\'");
  }

  #[test]
  fn a_single_quoted_scalar_spans_through_a_doubled_quote() {
    assert_eq!(value_text("k: 'it''s'\nz: 1\n"), "'it''s'");
  }

  #[test]
  fn an_unterminated_quoted_scalar_ends_at_the_span_end() {
    let yaml = "k: 'open";
    let span = Span::new(3, yaml.len());
    assert_eq!(quoted_end(yaml, span), yaml.len());
  }

  #[test]
  fn a_block_scalar_spans_its_content_and_only_the_lines_it_keeps() {
    assert_eq!(value_text("k: |\n  a\n\nz: 1\n"), "a");
    assert_eq!(value_text("k: >-\n  a\n  b\n\nz: 1\n"), "a\n  b");
    assert_eq!(value_text("k: |+\n  a\n\n\nz: 1\n"), "a\n\n");
    assert_eq!(value_text("k: |+\n  a\n    \n\n"), "a\n    \n");
    assert_eq!(value_text("k: |+\n  a\nz: 1\n"), "a");
    assert_eq!(value_text("k: |+\n  a\n\nz: 1\n"), "a\n");
  }

  fn value_texts(yaml: &str) -> Vec<(String, String)> {
    let src = ConfigSource::new(yaml, "test.yml");
    let root = build_tree(&src).expect("yaml parses");
    let NodeValue::Map(entries) = root.value else {
      panic!("root is not a map");
    };
    entries
      .into_iter()
      .map(|(k, v)| {
        let NodeValue::Scalar(key) = k.value else {
          panic!("key is not a scalar");
        };
        (key, yaml[v.span.start..v.span.end].to_string())
      })
      .collect()
  }

  #[test]
  fn an_empty_keep_chomped_block_scalar_does_not_span_the_lines_after_it() {
    for indicator in ["|+", ">+"] {
      let yaml = format!("k: {indicator}\n\n\nz: 1\ny: 2\nw: 3\n");
      let texts = value_texts(&yaml);
      let k = &texts[0].1;
      assert!(!k.contains('z') && !k.contains('y'), "{indicator}: {k:?}");
      assert_eq!(
        texts[1..],
        [
          ("z".to_string(), "1".to_string()),
          ("y".to_string(), "2".to_string()),
          ("w".to_string(), "3".to_string())
        ],
        "{indicator}"
      );

      let yaml = format!("k: {indicator}\n\n\n");
      assert_eq!(value_text(&yaml), format!("{indicator}\n\n\n"));
    }
  }

  #[test]
  fn a_flow_map_value_spans_to_its_closing_brace() {
    assert_eq!(value_text("k: {a: 1}\nz: 1\n"), "{a: 1}");
    assert_eq!(value_text("k: {a: 1}"), "{a: 1}");
  }

  #[test]
  fn a_flow_sequence_value_spans_to_its_closing_bracket() {
    assert_eq!(value_text("k: [a, b]\nz: 1\n"), "[a, b]");
    assert_eq!(value_text("k: [a, b]"), "[a, b]");
  }
}
