use std::fs::File;
use std::io::{Read, Write};
use std::os::fd::{FromRawFd, OwnedFd};
use std::process::{Command, Output, Stdio};

/// Runs `command` with stdin and stderr on a pseudo-terminal and stdout piped. Returns the piped output and everything the
/// command wrote to the terminal.
pub fn run_on_terminal(mut command: Command, keys: &[u8]) -> (Output, String) {
  let (master, slave) = open_raw_pty();
  let mut master = File::from(master);
  master.write_all(keys).unwrap();
  let child = command
    .stdin(Stdio::from(slave.try_clone().unwrap()))
    .stderr(Stdio::from(slave))
    .stdout(Stdio::piped())
    .spawn()
    .unwrap();
  drop(command);
  let mut reader = master.try_clone().unwrap();
  let drain = std::thread::spawn(move || {
    let mut screen = Vec::new();
    let _ = reader.read_to_end(&mut screen);
    String::from_utf8_lossy(&screen).into_owned()
  });
  let output = child.wait_with_output().unwrap();
  drop(master);
  let screen = drain.join().unwrap();
  (output, screen)
}

/// Asserts that each of `shown` is on `screen`, the terminal output of [`run_on_terminal`], before `question`.
pub fn assert_shown_before(screen: &str, question: &str, shown: &[&str]) {
  let asked = screen.find(question).expect("question on the terminal");
  for text in shown {
    let at = screen
      .find(text)
      .unwrap_or_else(|| panic!("{text:?} missing from the terminal: {screen:?}"));
    assert!(at < asked, "{text:?} comes after the question: {screen:?}");
  }
}

fn open_raw_pty() -> (OwnedFd, OwnedFd) {
  let (mut master, mut slave) = (0, 0);
  let mut size = libc::winsize {
    ws_row: 24,
    ws_col: 80,
    ws_xpixel: 0,
    ws_ypixel: 0,
  };
  assert_eq!(
    unsafe {
      libc::openpty(
        &raw mut master,
        &raw mut slave,
        std::ptr::null_mut(),
        std::ptr::null_mut(),
        &raw mut size,
      )
    },
    0
  );
  let (master, slave) = unsafe { (OwnedFd::from_raw_fd(master), OwnedFd::from_raw_fd(slave)) };
  let mut termios = std::mem::MaybeUninit::<libc::termios>::uninit();
  let fd = std::os::fd::AsRawFd::as_raw_fd(&slave);
  assert_eq!(unsafe { libc::tcgetattr(fd, termios.as_mut_ptr()) }, 0);
  let mut termios = unsafe { termios.assume_init() };
  unsafe { libc::cfmakeraw(&raw mut termios) };
  assert_eq!(unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw const termios) }, 0);
  (master, slave)
}
