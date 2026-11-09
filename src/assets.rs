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

    /// Record that a page was loaded while the bundle in `dir` is active,
    /// starting its trial (see
    /// [`HotswapMeta::trial_started`](crate::manifest::HotswapMeta::trial_started)).
    /// Called for every page (HTML) load, wherever the page itself is served
    /// from, so partial bundles start their trial too. Pages load rarely, so
    /// reading the small metadata file each time is cheap.
    fn record_trial_started(&self, dir: &Path) {
        let result = crate::updater::update_meta(dir, |meta| {
            let start = !meta.confirmed && !meta.trial_started;
            if start {
                meta.trial_started = true;
            }
            start
        });
        if let Err(e) = result {
            log::warn!("[hotswap] Failed to record trial start: {}", e);
        }
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
        let key_str = key.as_ref();
        // Read the current ota_dir from the shared handle.
        // This is re-read on every request so that apply/activate/rollback
        // take effect immediately without an app restart.
        if let Ok(guard) = self.ota_dir.read() {
            if let Some(ref dir) = *guard {
                log::debug!(
                    "[hotswap] Asset request: key={:?}, dir={}",
                    key_str,
                    dir.display()
                );
                if let Some(relative) = validate_asset_key(key_str) {
                    log::debug!("[hotswap] Validated key -> relative={:?}", relative);
                    // Try exact path
                    if let Some(data) = try_read(dir, relative) {
                        log::debug!("[hotswap] Serving from OTA: {}", relative);
                        if relative.ends_with(".html") {
                            self.record_trial_started(dir);
                        }
                        return Some(Cow::Owned(data));
                    }

                    // Try {path}.html fallback (matches Tauri's resolution chain)
                    let html_key = format!("{}.html", relative);
                    if let Some(data) = try_read(dir, &html_key) {
                        log::debug!("[hotswap] Serving from OTA (html fallback): {}", html_key);
                        self.record_trial_started(dir);
                        return Some(Cow::Owned(data));
                    }

                    // Try {path}/index.html fallback
                    let index_key = format!("{}/index.html", relative);
                    if let Some(data) = try_read(dir, &index_key) {
                        log::debug!("[hotswap] Serving from OTA (index fallback): {}", index_key);
                        self.record_trial_started(dir);
                        return Some(Cow::Owned(data));
                    }

                    log::debug!(
                        "[hotswap] OTA miss, falling back to embedded: {:?}",
                        relative
                    );
                } else {
                    log::debug!("[hotswap] Key validation failed for: {:?}", key_str);
                }
            }
        }

        // Fall back to embedded assets. A page served from there while an
        // update is active (a partial bundle without HTML) starts its trial too.
        let asset = self.embedded.get(key);
        if asset.is_some() && key_str.ends_with(".html") {
            if let Some(dir) = self.ota_dir.read().ok().and_then(|g| g.clone()) {
                self.record_trial_started(&dir);
            }
        }
        asset
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
    use tauri::Assets;

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

    // --- try_read tests ---

    #[test]
    fn test_try_read_existing_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("hello.txt"), b"world").unwrap();
        assert_eq!(try_read(dir.path(), "hello.txt"), Some(b"world".to_vec()));
    }

    #[test]
    fn test_try_read_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(try_read(dir.path(), "nope.txt"), None);
    }

    #[test]
    fn test_try_read_directory_not_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("subdir")).unwrap();
        assert_eq!(try_read(dir.path(), "subdir"), None);
    }

    #[test]
    fn test_try_read_nested_path() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("assets/css")).unwrap();
        std::fs::write(dir.path().join("assets/css/style.css"), b"body{}").unwrap();
        assert_eq!(
            try_read(dir.path(), "assets/css/style.css"),
            Some(b"body{}".to_vec())
        );
    }

    // --- HotswapAssets::get() tests ---

    /// Mock embedded assets that returns known data for specific keys.
    struct MockAssets {
        entries: std::collections::HashMap<String, Vec<u8>>,
    }

    impl MockAssets {
        fn new(entries: Vec<(&str, &[u8])>) -> Self {
            Self {
                entries: entries
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_vec()))
                    .collect(),
            }
        }
    }

    impl<R: Runtime> tauri::Assets<R> for MockAssets {
        fn get(&self, key: &AssetKey) -> Option<Cow<'_, [u8]>> {
            self.entries
                .get(key.as_ref())
                .map(|v| Cow::Borrowed(v.as_slice()))
        }

        fn iter(&self) -> Box<AssetsIter<'_>> {
            Box::new(std::iter::empty())
        }

        fn csp_hashes(&self, _html_path: &AssetKey) -> Box<dyn Iterator<Item = CspHash<'_>> + '_> {
            Box::new(std::iter::empty())
        }
    }

    type TestAssets = HotswapAssets<tauri::test::MockRuntime>;

    fn make_assets(ota_dir: AssetDirHandle, embedded_entries: Vec<(&str, &[u8])>) -> TestAssets {
        HotswapAssets::new(Box::new(MockAssets::new(embedded_entries)), ota_dir)
    }

    fn asset_key(s: &str) -> AssetKey {
        AssetKey::from(Path::new(s))
    }

    #[test]
    fn test_get_serves_from_ota_when_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.js"), b"ota-content").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![("/app.js", b"embedded-content")]);

        let result = assets.get(&asset_key("app.js"));
        assert!(result.is_some());
        // OTA content should be Cow::Owned
        let cow = result.unwrap();
        assert!(matches!(cow, Cow::Owned(_)));
        assert_eq!(cow.as_ref(), b"ota-content");
    }

    #[test]
    fn test_get_html_fallback() {
        let dir = tempfile::tempdir().unwrap();
        // No "about" file, but "about.html" exists
        std::fs::write(dir.path().join("about.html"), b"ota-about-html").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);

        let result = assets.get(&asset_key("about"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"ota-about-html");
    }

    #[test]
    fn test_get_index_html_fallback() {
        let dir = tempfile::tempdir().unwrap();
        // No "docs" file, no "docs.html", but "docs/index.html" exists
        std::fs::create_dir(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs/index.html"), b"ota-docs-index").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);

        let result = assets.get(&asset_key("docs"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"ota-docs-index");
    }

    #[test]
    fn test_get_all_fallbacks_miss_serves_embedded() {
        let dir = tempfile::tempdir().unwrap();
        // OTA dir exists but has nothing

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![("/missing.js", b"from-embedded")]);

        let result = assets.get(&asset_key("missing.js"));
        assert!(result.is_some());
        // Embedded returns Cow::Borrowed
        let cow = result.unwrap();
        assert!(matches!(cow, Cow::Borrowed(_)));
        assert_eq!(cow.as_ref(), b"from-embedded");
    }

    #[test]
    fn test_get_invalid_key_skips_ota() {
        let dir = tempfile::tempdir().unwrap();
        // Even if a file existed via traversal, it should be rejected
        std::fs::write(dir.path().join("secret.txt"), b"ota-secret").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![("../secret.txt", b"embedded-fallback")]);

        let result = assets.get(&asset_key("../secret.txt"));
        // validate_asset_key rejects "..", so it skips OTA and goes to embedded
        // AssetKey::from normalizes the path, so embedded lookup may or may not match.
        // The key point is that OTA is NOT consulted for traversal paths.
        // We verify by checking we don't get "ota-secret".
        if let Some(cow) = result {
            assert_ne!(cow.as_ref(), b"ota-secret" as &[u8]);
        }
    }

    #[test]
    fn test_get_ota_dir_none_serves_embedded() {
        let handle: AssetDirHandle = Arc::new(RwLock::new(None));
        let assets = make_assets(handle, vec![("/index.html", b"embedded-index")]);

        let result = assets.get(&asset_key("index.html"));
        assert!(result.is_some());
        assert_eq!(result.unwrap().as_ref(), b"embedded-index");
    }

    #[test]
    fn test_get_runtime_swap_of_ota_dir() {
        let dir_v1 = tempfile::tempdir().unwrap();
        std::fs::write(dir_v1.path().join("app.js"), b"version-1").unwrap();

        let dir_v2 = tempfile::tempdir().unwrap();
        std::fs::write(dir_v2.path().join("app.js"), b"version-2").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir_v1.path().to_path_buf())));
        let assets = make_assets(handle.clone(), vec![]);

        // First request serves from v1
        let result = assets.get(&asset_key("app.js"));
        assert_eq!(result.unwrap().as_ref(), b"version-1");

        // Swap the live_asset_dir to v2 (simulates activate/apply)
        {
            let mut guard = handle.write().unwrap();
            *guard = Some(dir_v2.path().to_path_buf());
        }

        // Next request should serve from v2 without recreating HotswapAssets
        let result = assets.get(&asset_key("app.js"));
        assert_eq!(result.unwrap().as_ref(), b"version-2");
    }

    #[test]
    fn test_get_runtime_swap_to_none() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.js"), b"ota-content").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle.clone(), vec![("/app.js", b"embedded-content")]);

        // Initially serves from OTA
        let result = assets.get(&asset_key("app.js"));
        assert_eq!(result.unwrap().as_ref(), b"ota-content");

        // Swap to None (simulates rollback to embedded)
        {
            let mut guard = handle.write().unwrap();
            *guard = None;
        }

        // Now should serve from embedded
        let result = assets.get(&asset_key("app.js"));
        assert_eq!(result.unwrap().as_ref(), b"embedded-content");
    }

    #[test]
    fn test_get_fallback_priority_exact_over_html() {
        let dir = tempfile::tempdir().unwrap();
        // Both "about" and "about.html" exist — exact match should win
        std::fs::write(dir.path().join("about"), b"exact-match").unwrap();
        std::fs::write(dir.path().join("about.html"), b"html-fallback").unwrap();

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);

        let result = assets.get(&asset_key("about"));
        assert_eq!(result.unwrap().as_ref(), b"exact-match");
    }

    fn write_unstarted_meta(dir: &Path) {
        std::fs::write(
            dir.join("hotswap-meta.json"),
            r#"{"version":"v2","sequence":2,"min_binary_version":"1.0.0","confirmed":false,"trial_started":false}"#,
        )
        .unwrap();
    }

    #[test]
    fn test_serving_a_page_starts_the_trial() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), b"<html>").unwrap();
        std::fs::write(dir.path().join("app.js"), b"js").unwrap();
        write_unstarted_meta(dir.path());

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);

        assets.get(&asset_key("app.js")).unwrap();
        assert!(
            !crate::updater::read_meta(dir.path()).unwrap().trial_started,
            "scripts alone (e.g. lazy chunks of the old page) don't start a trial"
        );
        assets.get(&asset_key("index.html")).unwrap();
        assert!(crate::updater::read_meta(dir.path()).unwrap().trial_started);
    }

    #[test]
    fn test_page_from_embedded_assets_starts_the_trial_of_a_partial_bundle() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("app.js"), b"js").unwrap(); // no index.html
        write_unstarted_meta(dir.path());

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![("/index.html", b"embedded-page")]);
        assert_eq!(
            assets.get(&asset_key("/index.html")).unwrap().as_ref(),
            b"embedded-page"
        );
        assert!(crate::updater::read_meta(dir.path()).unwrap().trial_started);
    }

    #[test]
    fn test_recreated_bundle_dir_starts_a_new_trial() {
        // The same seq dir can be deleted (rollback) and downloaded again.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("index.html"), b"<html>").unwrap();
        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);

        write_unstarted_meta(dir.path());
        assets.get(&asset_key("index.html")).unwrap();
        assert!(crate::updater::read_meta(dir.path()).unwrap().trial_started);

        write_unstarted_meta(dir.path()); // re-downloaded
        assets.get(&asset_key("index.html")).unwrap();
        assert!(crate::updater::read_meta(dir.path()).unwrap().trial_started);
    }

    #[test]
    fn test_index_fallback_starts_the_trial() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir(dir.path().join("settings")).unwrap();
        std::fs::write(dir.path().join("settings/index.html"), b"<html>").unwrap();
        write_unstarted_meta(dir.path());

        let handle: AssetDirHandle = Arc::new(RwLock::new(Some(dir.path().to_path_buf())));
        let assets = make_assets(handle, vec![]);
        assets.get(&asset_key("settings")).unwrap();
        assert!(crate::updater::read_meta(dir.path()).unwrap().trial_started);
    }
}
