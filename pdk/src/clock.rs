//! Monotonic clock access. `ctx.clock().start()` yields a `Timer` whose
//! `elapsed_ms()` re-reads the live clock — an honest stopwatch across an
//! arbitrarily long operation, with the plugin never touching raw nanos.

use crate::context::Host;

/// Clock handle, created via `ctx.clock()`.
pub struct Clock<'a> {
    host: &'a dyn Host,
}

impl<'a> Clock<'a> {
    pub(crate) fn new(host: &'a dyn Host) -> Self {
        Self { host }
    }
    /// Current monotonic reading in nanoseconds.
    pub fn now(&self) -> u64 {
        self.host.monotonic_nanos()
    }
    /// Start a stopwatch at the current instant.
    pub fn start(&self) -> Timer<'a> {
        Timer {
            host: self.host,
            start: self.host.monotonic_nanos(),
        }
    }
    /// Sleep for `ms` milliseconds. Backed by the host's monotonic sleep.
    pub fn sleep_ms(&self, ms: u64) {
        self.host.sleep_nanos(ms.saturating_mul(1_000_000));
    }
}

/// A running stopwatch: holds the host and start instant; `elapsed_ms`
/// re-reads the live clock, so it stays correct however long the gap.
pub struct Timer<'a> {
    host: &'a dyn Host,
    start: u64,
}

impl Timer<'_> {
    /// Elapsed milliseconds since `start`, saturating.
    pub fn elapsed_ms(&self) -> u64 {
        self.host.monotonic_nanos().saturating_sub(self.start) / 1_000_000
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::MockHost;
    use crate::Context;

    #[test]
    fn timer_elapsed_ms_from_scripted_clock() {
        // start() reads 0ns; elapsed_ms() reads 5_000_000ns -> 5ms.
        let host = MockHost::new().with_clock(&[0, 5_000_000]);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let t = ctx.clock().start();
        assert_eq!(t.elapsed_ms(), 5);
    }

    #[test]
    fn now_passes_through_and_saturates_non_increasing() {
        let host = MockHost::new().with_clock(&[10, 3]); // second reading < first
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let t = ctx.clock().start(); // reads 10
        assert_eq!(t.elapsed_ms(), 0); // (3 - 10) saturates to 0, never underflows
    }

    #[test]
    fn empty_clock_script_reads_zero() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert_eq!(ctx.clock().now(), 0);
    }

    #[test]
    fn sleep_ms_converts_to_nanos_and_records() {
        let host = MockHost::new();
        let ctx = Context::new(&host, "/w".into(), "s".into());
        ctx.clock().sleep_ms(5);
        ctx.clock().sleep_ms(0);
        assert_eq!(host.recorded_sleeps(), vec![5_000_000, 0]);
    }
}
