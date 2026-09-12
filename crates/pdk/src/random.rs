//! Randomness access. Routes through the host, which serves `wasi:random` on
//! the real host (a deterministic seed in tests). Funnelling through `Host`
//! keeps randomness mockable in native tests.

use crate::context::Host;

/// Randomness handle, created via `ctx.random()`.
pub struct Random<'a> {
    host: &'a dyn Host,
}

impl<'a> Random<'a> {
    pub(crate) fn new(host: &'a dyn Host) -> Self {
        Self { host }
    }
    /// `n` random bytes from the host (at least `n`).
    pub fn bytes(&self, n: usize) -> Vec<u8> {
        self.host.random_bytes(n)
    }
    /// A random UUIDv4 string (`8-4-4-4-12` lowercase hex).
    pub fn uuid(&self) -> String {
        let mut b = self.host.random_bytes(16);
        b.resize(16, 0); // defensive: contract is >= 16, but never index OOB
        b[6] = (b[6] & 0x0f) | 0x40; // version 4
        b[8] = (b[8] & 0x3f) | 0x80; // variant 1 (RFC 4122)
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7], b[8], b[9], b[10], b[11], b[12], b[13], b[14], b[15]
        )
    }
}

#[cfg(test)]
mod tests {
    use crate::testing::MockHost;
    use crate::Context;

    #[test]
    fn bytes_passthrough_returns_requested_length() {
        let host = MockHost::new().with_random(&[1, 2, 3]);
        let ctx = Context::new(&host, "/w".into(), "s".into());
        assert_eq!(ctx.random().bytes(5), vec![1, 2, 3, 1, 2]); // cycles the seed
    }

    #[test]
    fn uuid_is_v4_formatted_and_deterministic_under_mock() {
        let host = MockHost::new().with_random(&[0xab]); // every byte 0xab
        let ctx = Context::new(&host, "/w".into(), "s".into());
        let u = ctx.random().uuid();
        // 0xab: b[6]=(0x0b|0x40)=0x4b (version 4); b[8]=(0x2b|0x80)=0xab (variant 1)
        assert_eq!(u, "abababab-abab-4bab-abab-abababababab");
        assert_eq!(
            u,
            ctx.random().uuid(),
            "deterministic under a fixed mock seed"
        );
    }
}
