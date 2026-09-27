#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
  pub start: usize,
  pub end: usize,
}

impl Span {
  #[must_use]
  pub fn new(start: usize, end: usize) -> Self {
    Self { start, end }
  }

  #[must_use]
  pub fn point(at: usize) -> Self {
    Self { start: at, end: at }
  }

  #[must_use]
  pub fn to_source_span(self) -> miette::SourceSpan {
    (self.start, self.end.saturating_sub(self.start)).into()
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Spanned<T> {
  pub value: T,
  pub span: Span,
}

impl<T> Spanned<T> {
  #[must_use]
  pub fn new(value: T, span: Span) -> Self {
    Self { value, span }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn span_to_source_span_is_offset_and_len() {
    let ss = Span::new(3, 10).to_source_span();
    assert_eq!(ss.offset(), 3);
    assert_eq!(ss.len(), 7);
  }

  #[test]
  fn span_point_is_zero_length() {
    let ss = Span::point(5).to_source_span();
    assert_eq!(ss.offset(), 5);
    assert_eq!(ss.len(), 0);
  }
}
