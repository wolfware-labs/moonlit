//! Interior-mutable state shared between a plugin's middlewares.

use std::sync::Mutex;

/// A `Sync` cell for state shared between middlewares.
///
/// `moonlit_plugin! { state: T }` installs it in a `static`, and a `static`
/// must be `Sync`. `Shared<T>` is `Sync` whenever `T: Send`, which is what the
/// `Mutex` inside provides. Wasm is single-threaded, so it never contends.
///
/// # Examples
///
/// ```
/// use moonlit_pdk::state::Shared;
///
/// static SEEN: Shared<u32> = Shared::new(0);
///
/// SEEN.set(2);
/// assert_eq!(SEEN.update(|n| { *n += 1; *n }), 3);
/// assert_eq!(SEEN.get(), 3);
/// ```
pub struct Shared<T>(Mutex<T>);

impl<T> Shared<T> {
    /// Create a cell holding `value`.
    pub const fn new(value: T) -> Self {
        Self(Mutex::new(value))
    }
    /// Clone the current value out.
    pub fn get(&self) -> T
    where
        T: Clone,
    {
        self.0.lock().unwrap().clone()
    }
    /// Replace the value.
    pub fn set(&self, value: T) {
        *self.0.lock().unwrap() = value;
    }
    /// Mutate in place, returning whatever the closure returns.
    pub fn update<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        f(&mut self.0.lock().unwrap())
    }
}

impl<T: Default> Default for Shared<T> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_then_get_round_trips() {
        let s = Shared::new(7u32);
        assert_eq!(s.get(), 7);
    }

    #[test]
    fn set_overwrites() {
        let s = Shared::new(1u32);
        s.set(42);
        assert_eq!(s.get(), 42);
    }

    #[test]
    fn update_mutates_in_place_and_returns() {
        let s = Shared::new(10u32);
        let doubled = s.update(|v| {
            *v *= 2;
            *v
        });
        assert_eq!(doubled, 20);
        assert_eq!(s.get(), 20);
    }

    #[test]
    fn default_is_inner_default() {
        let s: Shared<Option<String>> = Shared::default();
        assert_eq!(s.get(), None);
    }

    #[test]
    fn models_latest_tag_to_commits_handoff() {
        // Exactly how `latest-tag` (writer) and `commits` (reader) will use it.
        let s: Shared<Option<String>> = Shared::default();
        assert_eq!(s.get(), None);
        s.set(Some("deadbeef".to_string()));
        assert_eq!(s.get(), Some("deadbeef".to_string()));
    }
}
