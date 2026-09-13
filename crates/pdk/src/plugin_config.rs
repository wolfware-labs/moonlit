//! Optional validation of a plugin's `config:` block, run at `init`.

/// Semantic validation for a plugin's `config:`, beyond what decoding checks.
///
/// `moonlit_plugin!` calls [`validate`](PluginConfig::validate) once the config
/// has decoded. The returned message surfaces verbatim as the `init` error,
/// where a decode failure would be wrapped in `"invalid plugin config: ..."`.
///
/// # Examples
///
/// ```
/// use moonlit_pdk::PluginConfig;
///
/// #[derive(Default)]
/// struct Cfg { token: String }
///
/// impl PluginConfig for Cfg {
///     fn validate(&self) -> Result<(), String> {
///         if self.token.is_empty() {
///             return Err("github: `token` is required".into());
///         }
///         Ok(())
///     }
/// }
///
/// assert_eq!(
///     Cfg::default().validate(),
///     Err("github: `token` is required".to_string())
/// );
/// ```
pub trait PluginConfig {
    /// Validate the decoded config.
    ///
    /// # Errors
    ///
    /// Return `Err(msg)` to fail `init`. `msg` is shown to the user unchanged.
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
