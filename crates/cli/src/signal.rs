//! Ctrl-C handling: first press requests graceful cancellation; the second aborts hard.

use tokio_util::sync::CancellationToken;

#[derive(Debug, PartialEq, Eq)]
pub enum SignalAction {
    /// First Ctrl-C: request cooperative cancellation.
    Cancel,
    /// Second Ctrl-C: abort the process immediately.
    HardAbort,
}

/// Decide what a given Ctrl-C (1-based `count`) should do.
pub fn decide(count: u32) -> SignalAction {
    if count <= 1 {
        SignalAction::Cancel
    } else {
        SignalAction::HardAbort
    }
}

/// Spawn a detached task that watches for Ctrl-C and drives `cancel`.
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
    fn first_cancels_second_aborts() {
        assert_eq!(decide(1), SignalAction::Cancel);
        assert_eq!(decide(2), SignalAction::HardAbort);
        assert_eq!(decide(3), SignalAction::HardAbort);
    }
}
