//! Optional semantic validation of a plugin's `config:`, run at `init`.
//!
//! The `moonlit_plugin!` macro calls [`PluginConfig::validate`] right after
//! decoding the plugin config. Unlike a decode error (wrapped as
//! `"invalid plugin config: …"`), the message returned here surfaces VERBATIM
//! as the init error, so a plugin can emit its exact 1.x contract string.

pub trait PluginConfig {
    /// Validate the decoded config. `Err(msg)` fails `init` with `msg` unwrapped.
    /// Default: accept everything.
    fn validate(&self) -> Result<(), String> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct Cfg {
        token: String,
    }
    impl PluginConfig for Cfg {
        fn validate(&self) -> Result<(), String> {
            if self.token.trim().is_empty() {
                return Err("token is required.".to_string());
            }
            Ok(())
        }
    }

    #[test]
    fn blank_value_returns_verbatim_message() {
        let msg = match (Cfg { token: "  ".into() }).validate() {
            Ok(()) => panic!("blank token must fail validation"),
            Err(e) => e,
        };
        assert_eq!(msg, "token is required.");
    }

    #[test]
    fn present_value_passes() {
        assert!((Cfg {
            token: "abc".into()
        })
        .validate()
        .is_ok());
    }

    #[test]
    fn default_impl_accepts() {
        struct Bare;
        impl PluginConfig for Bare {}
        assert!(Bare.validate().is_ok());
    }
}
