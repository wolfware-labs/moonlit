use tokio_util::sync::CancellationToken;

#[derive(Debug, PartialEq, Eq)]
pub enum SignalAction {
    Cancel,
    HardAbort,
}

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
