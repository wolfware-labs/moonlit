pub use moonlit_engine::sanitize::{display_safe, display_safe_text};

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
