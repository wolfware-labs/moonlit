#![allow(dead_code)]

use std::ops::{Deref, DerefMut};
use tempfile::TempDir;

pub struct Moonlit {
  command: assert_cmd::Command,
  _dirs: TempDir,
}

pub fn moonlit() -> Moonlit {
  let dirs = tempfile::tempdir().unwrap();
  let mut command = assert_cmd::Command::cargo_bin("moonlit").unwrap();
  command
    .env("MOONLIT_HOME", dirs.path().join("home"))
    .env("MOONLIT_CACHE_DIR", dirs.path().join("cache"));
  Moonlit { command, _dirs: dirs }
}

pub fn moonlit_process() -> (std::process::Command, TempDir) {
  let dirs = tempfile::tempdir().unwrap();
  let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_moonlit"));
  command
    .env("MOONLIT_HOME", dirs.path().join("home"))
    .env("MOONLIT_CACHE_DIR", dirs.path().join("cache"));
  (command, dirs)
}

impl Deref for Moonlit {
  type Target = assert_cmd::Command;

  fn deref(&self) -> &Self::Target {
    &self.command
  }
}

impl DerefMut for Moonlit {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.command
  }
}
