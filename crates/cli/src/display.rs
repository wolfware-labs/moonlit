use moonlit_engine::registry::VerificationStatus;
pub use moonlit_engine::sanitize::{display_safe, display_safe_text};

/// The registry's name for a verification state, as printed in JSON output.
#[must_use]
pub fn status_key(status: VerificationStatus) -> &'static str {
  match status {
    VerificationStatus::Verified => "verified",
    VerificationStatus::Unverified => "unverified",
    VerificationStatus::Failed => "failed",
    VerificationStatus::Pending => "pending",
  }
}

/// The label shown next to a plugin for its verification state.
#[must_use]
pub fn badge(status: VerificationStatus) -> &'static str {
  match status {
    VerificationStatus::Verified => "✓ verified",
    other => status_key(other),
  }
}
