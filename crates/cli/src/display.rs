/// Makes text from a plugin or a registry safe to print: control characters and bidirectional overrides
/// become visible escapes such as `\x1b`, so they cannot restyle, move or erase what the terminal shows.
#[must_use]
pub fn display_safe(text: &str) -> String {
  let mut out = String::with_capacity(text.len());
  for c in text.chars() {
    if let Some(escaped) = escape(c) {
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn escapes_c0_c1_and_delete() {
    assert_eq!(display_safe("a\u{1b}[8mb"), "a\\x1b[8mb");
    assert_eq!(display_safe("\u{9b}2K\u{7f}\u{7}"), "\\x9b2K\\x7f\\x07");
    assert_eq!(display_safe("line\nbreak\r\ttab"), "line\\x0abreak\\x0d\\x09tab");
  }

  #[test]
  fn escapes_bidirectional_overrides() {
    assert_eq!(display_safe("abc\u{202e}fed"), "abc\\u202efed");
    assert_eq!(display_safe("\u{2066}x\u{2069}"), "\\u2066x\\u2069");
  }

  #[test]
  fn leaves_ordinary_text_alone() {
    let text = "Connect to the internet 🌐 api.github.com — ✓ verified, ünïcode";
    assert_eq!(display_safe(text), text);
  }
}
