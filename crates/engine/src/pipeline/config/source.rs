#[derive(Clone, Copy)]
pub struct ConfigSource<'a> {
  pub yaml: &'a str,
  pub name: &'a str,
}

impl<'a> ConfigSource<'a> {
  #[must_use]
  pub fn new(yaml: &'a str, name: &'a str) -> Self {
    Self { yaml, name }
  }
}
