//! .NET version-string derivation shared by `build` and `pack`. Formulas match 1.x
//! exactly: `override ?? version.split('-')[0]` etc., with a whitespace-blank guard.

/// First `-`-delimited segment (strip prerelease): `1.2.3-rc.1` -> `1.2.3`.
fn strip_prerelease(v: &str) -> &str {
    v.split('-').next().unwrap_or(v)
}
/// First `+`-delimited segment (strip build metadata): `1.2.3+sha` -> `1.2.3`.
fn strip_metadata(v: &str) -> &str {
    v.split('+').next().unwrap_or(v)
}
/// 1.x `IsNullOrWhiteSpace` guard: a whitespace-only derived value is treated as absent.
fn non_blank(s: String) -> Option<String> {
    if s.trim().is_empty() {
        None
    } else {
        Some(s)
    }
}

/// AssemblyVersion / FileVersion: explicit override if present, else version minus
/// prerelease. (`configuration.AssemblyVersion ?? configuration.Version?.Split('-')[0]`.)
pub fn assembly_or_file_version(over: &Option<String>, version: &Option<String>) -> Option<String> {
    over.clone()
        .or_else(|| version.as_ref().map(|v| strip_prerelease(v).to_string()))
        .and_then(non_blank)
}

/// InformationalVersion: override if present, else the full version (no stripping).
pub fn informational_version(over: &Option<String>, version: &Option<String>) -> Option<String> {
    over.clone().or_else(|| version.clone()).and_then(non_blank)
}

/// PackageVersion: override if present, else version minus build metadata.
pub fn package_version(over: &Option<String>, version: &Option<String>) -> Option<String> {
    over.clone()
        .or_else(|| version.as_ref().map(|v| strip_metadata(v).to_string()))
        .and_then(non_blank)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(x: &str) -> Option<String> { Some(x.to_string()) }

    #[test]
    fn assembly_strips_prerelease_from_version() {
        assert_eq!(assembly_or_file_version(&None, &s("1.2.3-rc.1")), s("1.2.3"));
    }

    #[test]
    fn assembly_override_used_verbatim_without_split() {
        assert_eq!(assembly_or_file_version(&s("9.9.9-beta"), &s("1.2.3")), s("9.9.9-beta"));
    }

    #[test]
    fn assembly_none_when_no_version_and_no_override() {
        assert_eq!(assembly_or_file_version(&None, &None), None);
    }

    #[test]
    fn blank_override_is_rejected() {
        assert_eq!(assembly_or_file_version(&s("   "), &s("1.2.3")), None);
    }

    #[test]
    fn informational_uses_full_version() {
        assert_eq!(informational_version(&None, &s("1.2.3-rc.1+abc")), s("1.2.3-rc.1+abc"));
    }

    #[test]
    fn package_strips_build_metadata() {
        assert_eq!(package_version(&None, &s("1.2.3+sha.deadbeef")), s("1.2.3"));
    }
}
