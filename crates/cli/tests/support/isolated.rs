#![allow(dead_code, reason = "each test binary uses only some of these helpers")]

use std::ops::{Deref, DerefMut};
use std::path::PathBuf;

use tempfile::TempDir;

pub struct Moonlit {
  command: assert_cmd::Command,
  _dirs: TempDir,
}

pub fn moonlit() -> Moonlit {
  let (dirs, env) = isolated_env();
  let mut command = assert_cmd::Command::cargo_bin("moonlit").unwrap();
  command.envs(env);
  Moonlit { command, _dirs: dirs }
}

pub fn moonlit_process() -> (std::process::Command, TempDir) {
  let (dirs, env) = isolated_env();
  let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_moonlit"));
  command.envs(env);
  (command, dirs)
}

fn isolated_env() -> (TempDir, [(&'static str, PathBuf); 2]) {
  let dirs = tempfile::tempdir().unwrap();
  let env = [
    ("MOONLIT_HOME", dirs.path().join("home")),
    ("MOONLIT_CACHE_DIR", dirs.path().join("cache")),
  ];
  (dirs, env)
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
