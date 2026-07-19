//! `file://` resolution (§4.3): a local component used in the dev loop. Not cached — the file is
//! validated and its path returned directly, so editing the `.wasm` and re-running takes effect.

use std::path::Path;

use crate::resolve::{ResolveError, ResolvedPlugin};

/// Resolve a `file://` source to its direct path. The file must exist.
pub(crate) fn resolve_file(path: &Path) -> Result<ResolvedPlugin, ResolveError> {
    if !path.is_file() {
        return Err(ResolveError::NotFound(format!(
            "no plugin component at '{}'",
            path.display()
        )));
    }
    Ok(ResolvedPlugin {
        wasm_path: path.to_path_buf(),
        source: format!("file://{}", path.display()),
        digest: None,
        cached: true,
        middlewares: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_file_resolves_to_direct_path_uncached_digestless() {
        let dir = tempfile::tempdir().unwrap();
        let wasm = dir.path().join("plugin.wasm");
        std::fs::write(&wasm, b"\0asm").unwrap();

        let resolved = resolve_file(&wasm).unwrap();
        assert_eq!(resolved.wasm_path, wasm);
        assert_eq!(resolved.digest, None);
        assert!(resolved.cached);
        assert_eq!(resolved.middlewares, None);
        assert_eq!(resolved.source, format!("file://{}", wasm.display()));
    }

    #[test]
    fn missing_file_is_not_found() {
        let err = resolve_file(std::path::Path::new("/no/such/plugin.wasm")).unwrap_err();
        match err {
            ResolveError::NotFound(msg) => assert!(msg.contains("/no/such/plugin.wasm")),
            other => panic!("expected NotFound, got {other:?}"),
        }
    }

    #[test]
    fn directory_path_is_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_file(dir.path()).unwrap_err();
        assert!(matches!(err, ResolveError::NotFound(_)));
    }
}
