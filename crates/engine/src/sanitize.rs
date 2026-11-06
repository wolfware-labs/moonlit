/// Makes one line of text from a plugin or a registry safe to print: control characters, newlines and
/// bidirectional overrides become visible escapes such as `\x1b`, so they cannot restyle, move or erase
/// what the terminal shows.
#[must_use]
pub fn display_safe(text: &str) -> String {
  sanitize(text, false)
}

/// Like [`display_safe`] for a block of text that may span lines: `\n` and `\t` are kept, everything else
/// that could drive the terminal, `\r` included, becomes a visible escape.
#[must_use]
pub fn display_safe_text(text: &str) -> String {
  sanitize(text, true)
}

fn sanitize(text: &str, keep_layout: bool) -> String {
  let mut out = String::with_capacity(text.len());
  for c in text.chars() {
    if keep_layout && matches!(c, '\n' | '\t') {
      out.push(c);
    } else if let Some(escaped) = escape(c) {
      out.push_str(&escaped);
    } else {
      out.push(c);
    }
  }
  out
}

/// The visible escape for a character that must not reach the terminal as is; `None` for any other character.
#[must_use]
pub fn escape(c: char) -> Option<String> {
  let code = u32::from(c);
  if c.is_control() {
    Some(format!("\\x{code:02x}"))
  } else if is_bidi_control(c) {
    Some(format!("\\u{code:04x}"))
  } else {
    None
  }
}

fn is_bidi_control(c: char) -> bool {
  matches!(c, '\u{061c}' | '\u{200e}' | '\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2066}'..='\u{2069}')
}

/// Quotes a value as a YAML double-quoted scalar, escaping quotes, backslashes and any character that could
/// drive the terminal, so it parses back to the same value and prints safely.
#[must_use]
pub fn quote(value: &str) -> String {
  let mut escaped = String::with_capacity(value.len() + 2);
  escaped.push('"');
  for c in value.chars() {
    match c {
      '\\' => escaped.push_str("\\\\"),
      '"' => escaped.push_str("\\\""),
      '\n' => escaped.push_str("\\n"),
      '\r' => escaped.push_str("\\r"),
      '\t' => escaped.push_str("\\t"),
      c => match escape(c) {
        Some(code) => escaped.push_str(&code),
        None => escaped.push(c),
      },
    }
  }
  escaped.push('"');
  escaped
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn text_keeps_newline_and_tab_and_escapes_the_rest() {
    assert_eq!(
      display_safe_text("a\nb\tc\rd\u{1b}[2K\u{9b}\u{7f}"),
      "a\nb\tc\\x0dd\\x1b[2K\\x9b\\x7f"
    );
    assert_eq!(display_safe_text("x\u{202e}y"), "x\\u202ey");
  }

  #[test]
  fn line_variant_escapes_newline_and_tab() {
    assert_eq!(display_safe("a\nb\tc"), "a\\x0ab\\x09c");
  }
}
