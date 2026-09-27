use tokio_util::sync::CancellationToken;

#[derive(Debug, PartialEq, Eq)]
pub enum SignalAction {
  Cancel,
  HardAbort,
}

#[must_use]
pub fn decide(count: u32) -> SignalAction {
  if count <= 1 {
    SignalAction::Cancel
  } else {
    SignalAction::HardAbort
  }
}

pub fn spawn_watcher(cancel: CancellationToken) {
  tokio::spawn(async move {
    let mut count = 0u32;
    loop {
      if tokio::signal::ctrl_c().await.is_err() {
        return;
      }
      count += 1;
      match decide(count) {
        SignalAction::Cancel => {
          eprintln!("\nCancelling… (press Ctrl-C again to abort)");
          cancel.cancel();
        }
        SignalAction::HardAbort => std::process::exit(130),
      }
    }
  });
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn first_interrupt_cancels_and_later_ones_abort() {
    assert_eq!(decide(0), SignalAction::Cancel);
    assert_eq!(decide(1), SignalAction::Cancel);
    assert_eq!(decide(2), SignalAction::HardAbort);
    assert_eq!(decide(7), SignalAction::HardAbort);
  }

  #[tokio::test]
  async fn watcher_leaves_the_token_alone_without_a_signal() {
    let cancel = CancellationToken::new();
    spawn_watcher(cancel.clone());
    tokio::task::yield_now().await;
    assert!(!cancel.is_cancelled());
  }
}
