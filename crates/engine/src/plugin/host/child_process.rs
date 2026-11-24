use crate::plugin::host::auth::{build_globset, filter_env};
use crate::plugin::wit::moonlit::plugin::process::{Command, OutputChunk, StdioStream};
use std::path::{Component, Path, PathBuf};
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::sync::{mpsc, oneshot};

/// Host variables every child process gets, whatever the plugin was granted: enough to find programs and a home.
const BASELINE: &[&str] = &["PATH", "HOME", "TMPDIR", "LANG", "TZ", "USER", "LOGNAME"];

/// The Windows counterpart of [`BASELINE`]; names compare without case there.
const WINDOWS_BASELINE: &[&str] = &[
  "PATH",
  "PATHEXT",
  "COMSPEC",
  "SYSTEMROOT",
  "SYSTEMDRIVE",
  "WINDIR",
  "TEMP",
  "TMP",
  "USERPROFILE",
  "HOMEDRIVE",
  "HOMEPATH",
  "USERNAME",
  "APPDATA",
  "LOCALAPPDATA",
  "PROGRAMDATA",
  "PROGRAMFILES",
  "PROGRAMFILES(X86)",
];

/// Whether the host passes its own variable `name` to every child process.
fn in_baseline(name: &str, windows: bool) -> bool {
  if windows {
    WINDOWS_BASELINE.iter().any(|b| b.eq_ignore_ascii_case(name))
  } else {
    BASELINE.contains(&name) || name.starts_with("LC_")
  }
}

/// Whether a plugin may not set `name` for a program it runs: the variables that choose which program or which
/// libraries load.
#[must_use]
pub fn is_loader_variable(name: &str, windows: bool) -> bool {
  let upper = name.to_ascii_uppercase();
  let name = if windows { upper.as_str() } else { name };
  matches!(name, "PATH" | "GCONV_PATH" | "LOCPATH")
    || name.starts_with("LD_")
    || name.starts_with("DYLD_")
    || (windows && matches!(name, "PATHEXT" | "COMSPEC"))
}

/// Whether `name` can name one variable. An empty name, or one with `=`, would reach the child as part of
/// another variable (`PATH=x` set to `y` is read by a shell as `PATH` set to `x=y`); one with a NUL cannot be
/// passed at all.
#[must_use]
pub fn is_variable_name(name: &str) -> bool {
  !name.is_empty() && !name.contains(['=', '\0'])
}

/// Whether `program` names a path that climbs out of a folder with `..`.
#[must_use]
pub fn climbs(program: &str) -> bool {
  Path::new(program).components().any(|c| c == Component::ParentDir)
}

/// What a program a plugin runs may see of the host: its environment and the folder it runs in. Not `Debug`: the
/// environment holds granted secrets.
pub struct ChildSandbox {
  working_directory: PathBuf,
  env: Vec<(String, String)>,
  granted: globset::GlobSet,
}

impl ChildSandbox {
  /// The baseline variables of `snapshot`, then the ones `granted` (the plugin's `permissions.env`) matches.
  #[must_use]
  pub fn new(working_directory: &Path, granted: &[String], snapshot: &[(String, String)]) -> Self {
    let mut env: Vec<(String, String)> = snapshot
      .iter()
      .filter(|(name, _)| in_baseline(name, cfg!(windows)))
      .cloned()
      .collect();
    env.extend(filter_env(granted, snapshot));
    Self {
      working_directory: working_directory.to_path_buf(),
      env,
      granted: build_globset(granted),
    }
  }

  /// Whether the plugin's `permissions.env` covers `name`, so the plugin may set it for a program it runs.
  #[must_use]
  pub fn grants(&self, name: &str) -> bool {
    self.granted.is_match(name)
  }

  /// The folder a child runs in: the working directory, or `requested` when it is that folder or one inside it.
  /// `Ok(None)` means `requested` leads outside. Paths are resolved without the `\\?\` prefix on Windows, which
  /// `cmd.exe` refuses as a current directory.
  fn folder(&self, requested: Option<&str>) -> std::io::Result<Option<PathBuf>> {
    let root = dunce::canonicalize(&self.working_directory)?;
    let Some(requested) = requested else {
      return Ok(Some(root));
    };
    let folder = dunce::canonicalize(root.join(requested))?;
    Ok(folder.starts_with(&root).then_some(folder))
  }
}

/// Why a child process did not start.
#[derive(Debug, PartialEq, Eq)]
pub enum StartError {
  /// The folder the plugin asked for is outside the working directory.
  OutsideWorkingDirectory(String),
  /// Anything else, worded for the plugin.
  Failed(String),
}

/// The exit code reported for a child that gave none: one a signal stopped, or one whose exit was not read.
pub const UNKNOWN_EXIT: i32 = -1;

pub struct ChildProcess {
  pub rx: mpsc::Receiver<OutputChunk>,
  pub exit_rx: Option<oneshot::Receiver<i32>>,
  pub exit_cached: Option<i32>,
  pub kill_tx: Option<oneshot::Sender<()>>,
}

impl ChildProcess {
  pub fn start(command: &Command, sandbox: &ChildSandbox) -> Result<Self, StartError> {
    let requested = command.cwd.as_deref();
    let folder = match sandbox.folder(requested) {
      Ok(Some(folder)) => folder,
      Ok(None) => return Err(StartError::OutsideWorkingDirectory(requested.unwrap_or_default().to_string())),
      Err(e) => {
        let named = requested.unwrap_or("the working directory");
        return Err(StartError::Failed(format!("cannot run {} in {named}: {e}", command.program)));
      }
    };
    let mut c = tokio::process::Command::new(&command.program);
    c.args(&command.args);
    c.current_dir(folder);
    c.env_clear();
    c.envs(sandbox.env.iter().map(|(k, v)| (k, v)));
    c.envs(command.env.iter().map(|(k, v)| (k, v)));
    c.stdin(if command.stdin.is_some() {
      Stdio::piped()
    } else {
      Stdio::null()
    });
    c.stdout(Stdio::piped());
    c.stderr(Stdio::piped());

    let mut child = c
      .spawn()
      .map_err(|e| StartError::Failed(format!("failed to spawn {}: {e}", command.program)))?;

    if let (Some(input), Some(mut stdin)) = (command.stdin.clone(), child.stdin.take()) {
      tokio::spawn(async move {
        let _ = stdin.write_all(input.as_bytes()).await;
        let _ = stdin.shutdown().await;
      });
    }

    let stdout = child
      .stdout
      .take()
      .ok_or_else(|| StartError::Failed("no stdout pipe".to_string()))?;
    let stderr = child
      .stderr
      .take()
      .ok_or_else(|| StartError::Failed("no stderr pipe".to_string()))?;

    let (tx, rx) = mpsc::channel::<OutputChunk>(64);
    let (exit_tx, exit_rx) = oneshot::channel::<i32>();
    let (kill_tx, kill_rx) = oneshot::channel::<()>();

    tokio::spawn(Self::reader_task(child, stdout, stderr, tx, exit_tx, kill_rx));

    Ok(ChildProcess {
      rx,
      exit_rx: Some(exit_rx),
      exit_cached: None,
      kill_tx: Some(kill_tx),
    })
  }

  /// The child's exit code, [`UNKNOWN_EXIT`] when it gave none. Waits for the exit the first time and keeps the
  /// code.
  pub async fn wait_code(&mut self) -> i32 {
    if let Some(rx) = self.exit_rx.take() {
      self.exit_cached = Some(rx.await.unwrap_or(UNKNOWN_EXIT));
    }
    self.exit_cached.unwrap_or(UNKNOWN_EXIT)
  }

  async fn reader_task(
    mut child: tokio::process::Child,
    stdout: tokio::process::ChildStdout,
    stderr: tokio::process::ChildStderr,
    tx: mpsc::Sender<OutputChunk>,
    exit_tx: oneshot::Sender<i32>,
    mut kill_rx: oneshot::Receiver<()>,
  ) {
    let mut out = BufReader::new(stdout).lines();
    let mut err = BufReader::new(stderr).lines();
    let mut out_done = false;
    let mut err_done = false;

    while !(out_done && err_done) {
      tokio::select! {
        biased;
        _ = &mut kill_rx => { let _ = child.start_kill(); break; }
        line = out.next_line(), if !out_done => match line {
          Ok(Some(l)) => {
              if tx.send(OutputChunk { stream: StdioStream::Stdout, line: l }).await.is_err() { break; }
          }
          _ => out_done = true,
        },
        line = err.next_line(), if !err_done => match line {
          Ok(Some(l)) => {
              if tx.send(OutputChunk { stream: StdioStream::Stderr, line: l }).await.is_err() { break; }
          }
          _ => err_done = true,
        },
      }
    }

    drop(tx);
    let code = child.wait().await.ok().and_then(|s| s.code()).unwrap_or(UNKNOWN_EXIT);
    let _ = exit_tx.send(code);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  async fn collect(mut child: ChildProcess) -> (Vec<OutputChunk>, i32) {
    let mut chunks = Vec::new();
    while let Some(chunk) = child.rx.recv().await {
      chunks.push(chunk);
    }
    let code = child.exit_rx.take().unwrap().await.unwrap();
    (chunks, code)
  }

  fn lines(chunks: &[OutputChunk], stream: StdioStream) -> Vec<String> {
    chunks
      .iter()
      .filter(|c| std::mem::discriminant(&c.stream) == std::mem::discriminant(&stream))
      .map(|c| c.line.clone())
      .collect()
  }

  fn pair(name: &str, value: &str) -> (String, String) {
    (name.to_string(), value.to_string())
  }

  /// A host environment with this process's `PATH`, so `sh` is found, plus `extra`.
  fn host_env(extra: &[(&str, &str)]) -> Vec<(String, String)> {
    let mut env = vec![pair("PATH", &std::env::var("PATH").unwrap())];
    env.extend(extra.iter().map(|(k, v)| pair(k, v)));
    env
  }

  fn sh(script: &str) -> Command {
    Command {
      program: "sh".to_string(),
      args: vec!["-c".to_string(), script.to_string()],
      cwd: None,
      env: vec![],
      stdin: None,
    }
  }

  #[tokio::test]
  async fn passes_stdin_env_and_cwd_and_separates_stderr() {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir(dir.path().join("sub")).unwrap();
    std::fs::write(dir.path().join("sub").join("marker.txt"), b"").unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));
    let command = Command {
      cwd: Some("sub".to_string()),
      env: vec![pair("MOONLIT_CHILD_VAR", "from-env")],
      stdin: Some("from-stdin\n".to_string()),
      ..sh("cat; echo \"$MOONLIT_CHILD_VAR\"; echo oops >&2; ls")
    };

    let (chunks, code) = collect(ChildProcess::start(&command, &sandbox).unwrap()).await;

    let stdout = lines(&chunks, StdioStream::Stdout);
    assert_eq!(code, 0);
    assert_eq!(stdout, ["from-stdin", "from-env", "marker.txt"]);
    assert_eq!(lines(&chunks, StdioStream::Stderr), ["oops"]);
  }

  #[tokio::test]
  async fn a_child_sees_the_baseline_and_granted_variables_and_no_other_host_variable() {
    let dir = tempfile::tempdir().unwrap();
    let host = host_env(&[
      ("HOME", "/home/someone"),
      ("LC_ALL", "C"),
      ("GRANTED_TOKEN", "granted"),
      ("OTHER_SECRET", "secret"),
    ]);
    let sandbox = ChildSandbox::new(dir.path(), &["GRANTED_*".to_string()], &host);

    let script = "echo \"$HOME|$LC_ALL|$GRANTED_TOKEN|${OTHER_SECRET:-unset}\"";
    let (chunks, code) = collect(ChildProcess::start(&sh(script), &sandbox).unwrap()).await;

    assert_eq!(code, 0);
    assert_eq!(lines(&chunks, StdioStream::Stdout), ["/home/someone|C|granted|unset"]);
  }

  #[tokio::test]
  async fn a_child_does_not_see_a_variable_of_this_process() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));

    // CARGO_MANIFEST_DIR is set for every test process by cargo.
    assert!(std::env::var("CARGO_MANIFEST_DIR").is_ok());
    let script = "echo \"${CARGO_MANIFEST_DIR:-unset}\"";
    let (chunks, _) = collect(ChildProcess::start(&sh(script), &sandbox).unwrap()).await;

    assert_eq!(lines(&chunks, StdioStream::Stdout), ["unset"]);
  }

  #[tokio::test]
  async fn a_plugin_variable_replaces_a_host_one_of_the_same_name() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[("HOME", "/home/someone")]));
    let command = Command {
      env: vec![pair("HOME", "/plugin/home")],
      ..sh("echo \"$HOME\"")
    };

    let (chunks, _) = collect(ChildProcess::start(&command, &sandbox).unwrap()).await;

    assert_eq!(lines(&chunks, StdioStream::Stdout), ["/plugin/home"]);
  }

  #[cfg(unix)]
  #[tokio::test]
  async fn a_child_runs_in_the_working_directory_when_the_plugin_names_no_folder() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));

    let (chunks, _) = collect(ChildProcess::start(&sh("pwd -P"), &sandbox).unwrap()).await;

    let expected = dir.path().canonicalize().unwrap().display().to_string();
    assert_eq!(lines(&chunks, StdioStream::Stdout), [expected]);
  }

  #[test]
  fn a_folder_outside_the_working_directory_is_refused() {
    let outer = tempfile::tempdir().unwrap();
    let work = outer.path().join("work");
    std::fs::create_dir(&work).unwrap();
    std::fs::create_dir(outer.path().join("work-sibling")).unwrap();
    let sandbox = ChildSandbox::new(&work, &[], &host_env(&[]));
    let outside = outer.path().display().to_string();
    let sibling = outer.path().join("work-sibling").display().to_string();

    for cwd in ["..", "../work-sibling", outside.as_str(), sibling.as_str()] {
      let command = Command {
        cwd: Some(cwd.to_string()),
        ..sh("true")
      };
      let result = ChildProcess::start(&command, &sandbox).map(|_| ());
      assert_eq!(result, Err(StartError::OutsideWorkingDirectory(cwd.to_string())), "{cwd}");
    }
  }

  #[cfg(unix)]
  #[test]
  fn a_link_that_leads_outside_the_working_directory_is_refused() {
    let outer = tempfile::tempdir().unwrap();
    let work = outer.path().join("work");
    std::fs::create_dir(&work).unwrap();
    std::os::unix::fs::symlink(outer.path(), work.join("out")).unwrap();
    let sandbox = ChildSandbox::new(&work, &[], &host_env(&[]));
    let command = Command {
      cwd: Some("out".to_string()),
      ..sh("true")
    };

    let result = ChildProcess::start(&command, &sandbox).map(|_| ());

    assert_eq!(result, Err(StartError::OutsideWorkingDirectory("out".to_string())));
  }

  #[tokio::test]
  async fn the_working_directory_itself_and_its_absolute_path_are_accepted() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));
    let absolute = dir.path().display().to_string();

    for cwd in [".", absolute.as_str()] {
      let command = Command {
        cwd: Some(cwd.to_string()),
        ..sh("true")
      };
      let (_, code) = collect(ChildProcess::start(&command, &sandbox).unwrap()).await;
      assert_eq!(code, 0, "{cwd}");
    }
  }

  #[test]
  fn a_missing_folder_fails_to_start_and_names_the_folder() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));
    let command = Command {
      cwd: Some("no-such-folder".to_string()),
      ..sh("true")
    };

    let result = ChildProcess::start(&command, &sandbox).map(|_| ());

    assert!(
      matches!(&result, Err(StartError::Failed(msg)) if msg.starts_with("cannot run sh in no-such-folder: ")),
      "{result:?}"
    );
  }

  #[test]
  fn a_missing_program_fails_to_start() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &[], &host_env(&[]));
    let command = Command {
      program: "moonlit-no-such-program".to_string(),
      ..sh("true")
    };

    let result = ChildProcess::start(&command, &sandbox).map(|_| ());

    assert!(
      matches!(&result, Err(StartError::Failed(msg)) if msg.starts_with("failed to spawn moonlit-no-such-program")),
      "{result:?}"
    );
  }

  #[test]
  fn the_sandbox_grants_the_names_the_env_permission_covers() {
    let dir = tempfile::tempdir().unwrap();
    let sandbox = ChildSandbox::new(dir.path(), &["GIT_*".to_string(), "CI".to_string()], &[]);

    for name in ["GIT_AUTHOR_NAME", "GIT_", "CI"] {
      assert!(sandbox.grants(name), "{name}");
    }
    for name in ["GIT", "CI_JOB", "HOME", "git_author_name"] {
      assert!(!sandbox.grants(name), "{name}");
    }
  }

  #[test]
  fn the_baseline_is_a_fixed_list_per_platform() {
    for name in [
      "PATH", "HOME", "TMPDIR", "LANG", "TZ", "USER", "LOGNAME", "LC_ALL", "LC_CTYPE",
    ] {
      assert!(in_baseline(name, false), "{name}");
    }
    for name in ["Path", "home", "LC", "XLC_ALL", "SYSTEMROOT", "GITHUB_TOKEN", "LD_PRELOAD"] {
      assert!(!in_baseline(name, false), "{name}");
    }
    for name in ["Path", "SystemRoot", "USERPROFILE", "AppData", "ProgramFiles(x86)", "ComSpec"] {
      assert!(in_baseline(name, true), "{name}");
    }
    for name in ["HOME", "LC_ALL", "GITHUB_TOKEN"] {
      assert!(!in_baseline(name, true), "{name}");
    }
  }

  #[test]
  fn loader_variables_are_the_ones_that_choose_programs_and_libraries() {
    for name in [
      "PATH",
      "LD_PRELOAD",
      "LD_LIBRARY_PATH",
      "DYLD_INSERT_LIBRARIES",
      "GCONV_PATH",
      "LOCPATH",
    ] {
      assert!(is_loader_variable(name, false), "{name}");
      assert!(is_loader_variable(name, true), "{name}");
    }
    for name in [
      "Path",
      "ld_preload",
      "PATHEXT",
      "COMSPEC",
      "GIT_PATH",
      "MY_LD_FLAG",
      "XDYLD_X",
      "LD",
      "HOME",
    ] {
      assert!(!is_loader_variable(name, false), "{name}");
    }
    for name in ["Path", "ld_preload", "PathExt", "ComSpec", "dyld_x"] {
      assert!(is_loader_variable(name, true), "{name}");
    }
    for name in ["GIT_PATH", "HOME", "PATHS"] {
      assert!(!is_loader_variable(name, true), "{name}");
    }
  }

  #[test]
  fn a_variable_name_is_not_empty_and_has_no_equals_sign() {
    for name in ["A", "GIT_AUTHOR_NAME", "lower", "with space", "ProgramFiles(x86)"] {
      assert!(is_variable_name(name), "{name}");
    }
    for name in ["", "PATH=x", "=", "=C:", "A=", "A\0B"] {
      assert!(!is_variable_name(name), "{name:?}");
    }
  }

  #[test]
  fn a_program_path_climbs_when_it_has_a_parent_component() {
    for program in ["../sh", "bin/../../sh", "/usr/bin/../../tmp/x", ".."] {
      assert!(climbs(program), "{program}");
    }
    for program in ["git", "./run.sh", "/usr/bin/git", "bin/tool", "a..b", "..x/tool"] {
      assert!(!climbs(program), "{program}");
    }
  }
}
