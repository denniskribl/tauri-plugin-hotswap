use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, RwLock};
use tauri::utils::assets::{AssetKey, AssetsIter, CspHash};
use tauri::Runtime;

/// Shared handle to the active asset directory.
/// Updated by `applyUpdate()` / `activateUpdate()` / `rollback()` commands
/// so that `window.location.reload()` immediately serves the new assets
/// without requiring an app restart.
pub type AssetDirHandle = Arc<RwLock<Option<PathBuf>>>;

/// Custom Assets implementation that checks the filesystem first,
/// then falls back to the embedded assets from the binary.
pub struct HotswapAssets<R: Runtime> {
    /// The original embedded assets compiled into the binary.
    embedded: Box<dyn tauri::Assets<R>>,
    /// Shared handle to the active asset directory.
    /// `None` → serve embedded assets only.
    /// `Some(path)` → try filesystem first, fall back to embedded.
    ota_dir: AssetDirHandle,
}

impl<R: Runtime> HotswapAssets<R> {
    /// Create a new asset provider.
    ///
    /// The `ota_dir` handle is shared with `HotswapState` so that commands
    /// can update the active directory at runtime.
    pub fn new(embedded: Box<dyn tauri::Assets<R>>, ota_dir: AssetDirHandle) -> Self {
        if let Ok(guard) = ota_dir.read() {
            if let Some(ref path) = *guard {
                log::info!("[hotswap] Serving assets from: {}", path.display());
            } else {
                log::info!("[hotswap] No cached assets found, using embedded assets");
            }
        }
        Self { embedded, ota_dir }
    }
}

/// Validate an asset key and return the sanitized relative path.
/// Returns None if the key contains unsafe components.
fn validate_asset_key(key: &str) -> Option<&str> {
    let relative = key.trim_start_matches('/');
    if relative.is_empty() {
        return None;
    }

    // Reject any component that is not a normal filename
    let path = Path::new(relative);
    for component in path.components() {
        match component {
            Component::Normal(_) => {}
            // Reject ParentDir (..), CurDir (.), RootDir (/), Prefix (C:\)
            _ => return None,
        }
    }

    Some(relative)
}

/// Try to read a file from a directory. Returns the contents if found.
fn try_read(dir: &Path, relative: &str) -> Option<Vec<u8>> {
    let path = dir.join(relative);
    if path.is_file() {
        std::fs::read(&path).ok()
    } else {
        None
    }
}

impl<R: Runtime> tauri::Assets<R> for HotswapAssets<R> {
    fn setup(&self, app: &tauri::App<R>) {
        self.embedded.setup(app);
    }

    fn get(&self, key: &AssetKey) -> Option<Cow<'_, [u8]>> {
        // Read the current ota_dir from the shared handle.
        // This is re-read on every request so that apply/activate/rollback
        // take effect immediately without an app restart.
        if let Ok(guard) = self.ota_dir.read() {
            if let Some(ref dir) = *guard {
                if let Some(relative) = validate_asset_key(key.as_ref()) {
                    // Try exact path
                    if let Some(data) = try_read(dir, relative) {
                        return Some(Cow::Owned(data));
                    }

                    // Try {path}.html fallback (matches Tauri's resolution chain)
                    let html_key = format!("{}.html", relative);
                    if let Some(data) = try_read(dir, &html_key) {
                        return Some(Cow::Owned(data));
                    }

                    // Try {path}/index.html fallback
                    let index_key = format!("{}/index.html", relative);
                    if let Some(data) = try_read(dir, &index_key) {
                        return Some(Cow::Owned(data));
                    }
                }
            }
        }

        // Fall back to embedded assets
        self.embedded.get(key)
    }

    fn iter(&self) -> Box<AssetsIter<'_>> {
        self.embedded.iter()
    }

    fn csp_hashes(&self, html_path: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
        self.embedded.csp_hashes(html_path)
    }
}

/// Empty assets implementation used only as a temporary placeholder
/// during the context asset swap. Never serves actual requests.
pub(crate) struct EmptyAssets;

impl<R: Runtime> tauri::Assets<R> for EmptyAssets {
    fn get(&self, _key: &AssetKey) -> Option<Cow<'_, [u8]>> {
        None
    }

    fn iter(&self) -> Box<AssetsIter<'_>> {
        Box::new(std::iter::empty())
    }

    fn csp_hashes(&self, _html_path: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
        Box::new(std::iter::empty())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_asset_key_normal() {
        assert_eq!(validate_asset_key("/index.html"), Some("index.html"));
        assert_eq!(validate_asset_key("index.html"), Some("index.html"));
    }

    #[test]
    fn test_validate_asset_key_nested() {
        assert_eq!(
            validate_asset_key("/assets/css/style.css"),
            Some("assets/css/style.css")
        );
    }

    #[test]
    fn test_validate_asset_key_rejects_traversal() {
        assert!(validate_asset_key("/../../../etc/passwd").is_none());
        assert!(validate_asset_key("/foo/../../etc/passwd").is_none());
        assert!(validate_asset_key("../escape").is_none());
    }

    #[test]
    fn test_validate_asset_key_rejects_empty() {
        assert!(validate_asset_key("/").is_none());
        assert!(validate_asset_key("").is_none());
    }

    #[test]
    fn test_validate_asset_key_rejects_curdir() {
        assert!(validate_asset_key("./file.txt").is_none());
    }
}
