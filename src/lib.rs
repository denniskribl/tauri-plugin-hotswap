//! # tauri-plugin-hotswap
//!
//! Hot-swap frontend assets at runtime without rebuilding the binary.
//!
//! This Tauri v2 plugin swaps the embedded asset provider at startup via
//! `Context::set_assets()`. The WebView keeps loading from `tauri://localhost` —
//! the swap is transparent. If no cached bundle is available, embedded assets
//! are served as usual.
//!
//! ## Quick start
//!
//! ```json
//! // tauri.conf.json
//! {
//!   "plugins": {
//!     "hotswap": {
//!       "endpoint": "https://example.com/api/ota/{{current_sequence}}",
//!       "pubkey": "<YOUR_MINISIGN_PUBKEY>"
//!     }
//!   }
//! }
//! ```
//!
//! ```rust,ignore
//! let context = tauri::generate_context!();
//! let (plugin, context) = tauri_plugin_hotswap::init(context)
//!     .expect("failed to initialize hotswap plugin");
//!
//! tauri::Builder::default()
//!     .plugin(plugin)
//!     .run(context)
//!     .expect("error running app");
//! ```

#![warn(missing_docs)]

mod assets;
mod commands;
/// Error types returned by the plugin.
pub mod error;
/// Manifest and response types exchanged between client and server.
pub mod manifest;
/// Resolver trait and built-in implementations for update checking.
pub mod resolver;
mod updater;

use std::collections::HashMap;
use std::fmt;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, RwLock};

use assets::{AssetDirHandle, EmptyAssets};
use serde::{Deserialize, Serialize};
use tauri::{
    plugin::{Builder, TauriPlugin},
    Manager, Runtime,
};

// Re-export all public types at the crate root for convenience.
pub use assets::HotswapAssets;
pub use error::Error;
pub use manifest::{HotswapCheckResult, HotswapManifest, HotswapMeta, HotswapVersionInfo};
pub use resolver::{CheckContext, HotswapResolver, HttpResolver, StaticFileResolver};
pub use updater::{DownloadProgress, LifecycleEvent};

/// Configuration that can be specified in `tauri.conf.json` under
/// `plugins.hotswap`, or passed programmatically.
///
/// # Example (tauri.conf.json)
///
/// ```json
/// {
///   "plugins": {
///     "hotswap": {
///       "endpoint": "https://example.com/api/ota/{{current_sequence}}",
///       "pubkey": "<YOUR_MINISIGN_PUBKEY>",
///       "channel": "production",
///       "headers": { "Authorization": "Bearer <token>" }
///     }
///   }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HotswapConfig {
    /// The update check endpoint URL.
    /// Use `{{current_sequence}}` as a placeholder.
    pub endpoint: Option<String>,

    /// The minisign public key (RW... base64 line).
    pub pubkey: String,

    /// Maximum allowed bundle size in bytes. Default: 512 MB.
    #[serde(default)]
    pub max_bundle_size: Option<u64>,

    /// Whether to reject non-HTTPS URLs. Default: true.
    #[serde(default)]
    pub require_https: Option<bool>,

    /// Whether to discard cache on binary upgrade. Default: true.
    #[serde(default)]
    pub discard_on_binary_upgrade: Option<bool>,

    /// Custom HTTP headers sent on check and download requests.
    /// Use for auth tokens, API keys, etc.
    #[serde(default)]
    pub headers: Option<HashMap<String, String>>,

    /// Update channel (e.g. "production", "staging", "beta").
    /// Sent as a query param on check requests. Can be changed at
    /// runtime via `configure()`.
    #[serde(default)]
    pub channel: Option<String>,

    /// Maximum download retry attempts with exponential backoff. Default: 3.
    #[serde(default)]
    pub max_retries: Option<u32>,
}

impl HotswapConfig {
    /// Create a new config with the given public key and sensible defaults.
    pub fn new(pubkey: impl Into<String>) -> Self {
        Self {
            endpoint: None,
            pubkey: pubkey.into(),
            max_bundle_size: None,
            require_https: None,
            discard_on_binary_upgrade: None,
            headers: None,
            channel: None,
            max_retries: None,
        }
    }

    /// Set the update check endpoint URL.
    pub fn endpoint(mut self, url: impl Into<String>) -> Self {
        self.endpoint = Some(url.into());
        self
    }

    /// Set the update channel.
    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Add a custom header sent on every request.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers
            .get_or_insert_with(HashMap::new)
            .insert(key.into(), value.into());
        self
    }
}

/// Plugin state managed by Tauri, accessible from commands.
pub(crate) struct HotswapState {
    pub(crate) resolver: Box<dyn HotswapResolver>,
    pub(crate) pubkey: String,
    pub(crate) binary_version: String,
    pub(crate) base_dir: PathBuf,
    pub(crate) max_bundle_size: u64,
    pub(crate) require_https: bool,
    pub(crate) max_retries: u32,
    pub(crate) http_client: reqwest::Client,
    pub(crate) custom_headers: Mutex<HashMap<String, String>>,
    pub(crate) channel: Mutex<Option<String>>,
    pub(crate) endpoint_override: Mutex<Option<String>>,
    pub(crate) current_sequence: Mutex<u64>,
    pub(crate) current_version: Mutex<Option<String>>,
    pub(crate) pending_manifest: Mutex<Option<HotswapManifest>>,
    /// Shared handle to the live asset directory used by `HotswapAssets`.
    /// Updated by apply/activate/rollback so `window.location.reload()`
    /// immediately serves the new assets without an app restart.
    pub(crate) live_asset_dir: AssetDirHandle,
}

impl fmt::Debug for HotswapState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("HotswapState")
            .field("binary_version", &self.binary_version)
            .field("base_dir", &self.base_dir)
            .field("max_bundle_size", &self.max_bundle_size)
            .field("require_https", &self.require_https)
            .field("max_retries", &self.max_retries)
            .field("channel", &self.channel)
            .field("current_sequence", &self.current_sequence)
            .field("current_version", &self.current_version)
            .finish_non_exhaustive()
    }
}

/// Initialize the plugin by reading config from `tauri.conf.json`.
///
/// Returns an error if the config is missing, invalid, or the endpoint
/// URL violates `require_https`.
pub fn init<R: Runtime>(
    context: tauri::Context<R>,
) -> Result<(TauriPlugin<R>, tauri::Context<R>), Error> {
    let config: HotswapConfig = context
        .config()
        .plugins
        .0
        .get("hotswap")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or_else(|| {
            Error::Config("missing or invalid 'plugins.hotswap' in tauri.conf.json".into())
        })?;

    let endpoint = config
        .endpoint
        .clone()
        .ok_or_else(|| Error::Config("'endpoint' is required in plugins.hotswap config".into()))?;

    let mut resolver = HttpResolver::new(endpoint);
    if let Some(ref headers) = config.headers {
        resolver = resolver.with_headers(headers.clone());
    }

    build_plugin(context, Box::new(resolver), config)
}

/// Initialize with explicit config (no tauri.conf.json needed).
///
/// Returns an error if the endpoint URL violates `require_https`.
pub fn init_with_config<R: Runtime>(
    context: tauri::Context<R>,
    config: HotswapConfig,
) -> Result<(TauriPlugin<R>, tauri::Context<R>), Error> {
    let endpoint = config
        .endpoint
        .clone()
        .ok_or_else(|| Error::Config("'endpoint' is required in HotswapConfig".into()))?;

    let mut resolver = HttpResolver::new(endpoint);
    if let Some(ref headers) = config.headers {
        resolver = resolver.with_headers(headers.clone());
    }

    build_plugin(context, Box::new(resolver), config)
}

/// Builder for advanced usage with a custom resolver.
///
/// # Example
///
/// ```rust,ignore
/// let (plugin, context) = tauri_plugin_hotswap::HotswapBuilder::new("<YOUR_MINISIGN_PUBKEY>")
///     .resolver(tauri_plugin_hotswap::StaticFileResolver::new(
///         "https://cdn.example.com/ota/latest.json",
///     ))
///     .channel("production")
///     .header("Authorization", "Bearer <token>")
///     .build(context)
///     .expect("failed to init hotswap");
/// ```
pub struct HotswapBuilder {
    pubkey: String,
    resolver: Option<Box<dyn HotswapResolver>>,
    max_bundle_size: u64,
    require_https: bool,
    discard_on_binary_upgrade: bool,
    headers: HashMap<String, String>,
    channel: Option<String>,
    max_retries: u32,
}

impl HotswapBuilder {
    /// Create a new builder with the given minisign public key.
    pub fn new(pubkey: impl Into<String>) -> Self {
        Self {
            pubkey: pubkey.into(),
            resolver: None,
            max_bundle_size: updater::DEFAULT_MAX_BUNDLE_SIZE,
            require_https: true,
            discard_on_binary_upgrade: true,
            headers: HashMap::new(),
            channel: None,
            max_retries: updater::DEFAULT_MAX_RETRIES,
        }
    }

    /// Set the update resolver.
    pub fn resolver(mut self, resolver: impl HotswapResolver) -> Self {
        self.resolver = Some(Box::new(resolver));
        self
    }

    /// Set the maximum allowed bundle size in bytes. Default: 512 MB.
    pub fn max_bundle_size(mut self, bytes: u64) -> Self {
        self.max_bundle_size = bytes;
        self
    }

    /// Whether to reject non-HTTPS URLs. Default: true.
    pub fn require_https(mut self, require: bool) -> Self {
        self.require_https = require;
        self
    }

    /// Whether to discard cache when the binary is upgraded. Default: true.
    pub fn discard_on_binary_upgrade(mut self, discard: bool) -> Self {
        self.discard_on_binary_upgrade = discard;
        self
    }

    /// Add a custom header sent on download requests.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }

    /// Set the update channel.
    pub fn channel(mut self, channel: impl Into<String>) -> Self {
        self.channel = Some(channel.into());
        self
    }

    /// Set the maximum download retry attempts. Default: 3.
    pub fn max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    /// Build the plugin. Returns an error if configuration is invalid.
    pub fn build<R: Runtime>(
        self,
        context: tauri::Context<R>,
    ) -> Result<(TauriPlugin<R>, tauri::Context<R>), Error> {
        let resolver = self
            .resolver
            .ok_or_else(|| Error::Config("a resolver must be set via .resolver()".into()))?;

        let config = HotswapConfig {
            endpoint: None,
            pubkey: self.pubkey,
            max_bundle_size: Some(self.max_bundle_size),
            require_https: Some(self.require_https),
            discard_on_binary_upgrade: Some(self.discard_on_binary_upgrade),
            headers: Some(self.headers),
            channel: self.channel,
            max_retries: Some(self.max_retries),
        };

        build_plugin(context, resolver, config)
    }
}

fn build_plugin<R: Runtime>(
    mut context: tauri::Context<R>,
    resolver: Box<dyn HotswapResolver>,
    config: HotswapConfig,
) -> Result<(TauriPlugin<R>, tauri::Context<R>), Error> {
    let binary_version = context.config().version.clone().unwrap_or_default();
    let app_id = context.config().identifier.clone();
    let base_dir = resolve_base_dir(&app_id);
    let max_bundle_size = config
        .max_bundle_size
        .unwrap_or(updater::DEFAULT_MAX_BUNDLE_SIZE);
    let require_https = config.require_https.unwrap_or(true);
    let discard_on_upgrade = config.discard_on_binary_upgrade.unwrap_or(true);
    let custom_headers = config.headers.unwrap_or_default();
    let channel = config.channel.clone();
    let max_retries = config.max_retries.unwrap_or(updater::DEFAULT_MAX_RETRIES);

    if binary_version.is_empty() {
        log::warn!(
            "[hotswap] No 'version' set in tauri.conf.json. \
             Binary compatibility checks will not work correctly."
        );
    }

    if require_https {
        if let Some(ref endpoint) = config.endpoint {
            if !endpoint.starts_with("https://") {
                return Err(Error::InsecureUrl(endpoint.clone()));
            }
        }
    }

    let _ = std::fs::create_dir_all(&base_dir);

    let ota_dir = updater::check_compatibility(&base_dir, &binary_version, discard_on_upgrade);
    let meta = ota_dir.as_ref().and_then(|d| updater::read_meta(d));
    let current_sequence = meta.as_ref().map(|m| m.sequence).unwrap_or(0);
    let current_version = meta.map(|m| m.version);

    // Shared handle: HotswapAssets reads from this on every request,
    // and commands (apply/activate/rollback) update it at runtime.
    let live_asset_dir: AssetDirHandle = Arc::new(RwLock::new(ota_dir));

    let embedded: Box<dyn tauri::Assets<R>> =
        std::mem::replace(&mut context.assets, Box::new(EmptyAssets));
    context.assets = Box::new(HotswapAssets::new(embedded, Arc::clone(&live_asset_dir)));

    let pubkey = config.pubkey.clone();
    let binary_version_clone = binary_version.clone();
    let base_dir_clone = base_dir.clone();
    let current_sequence_clone = current_sequence;
    let current_version_clone = current_version.clone();
    let http_client = reqwest::Client::new();

    let plugin = Builder::new("hotswap")
        .invoke_handler(tauri::generate_handler![
            commands::hotswap_check,
            commands::hotswap_apply,
            commands::hotswap_download,
            commands::hotswap_activate,
            commands::hotswap_rollback,
            commands::hotswap_current_version,
            commands::hotswap_notify_ready,
            commands::hotswap_configure,
            commands::hotswap_get_config,
        ])
        .setup(move |app, _api| {
            app.manage(HotswapState {
                resolver,
                pubkey,
                binary_version: binary_version_clone,
                base_dir: base_dir_clone,
                max_bundle_size,
                require_https,
                max_retries,
                http_client,
                custom_headers: Mutex::new(custom_headers),
                channel: Mutex::new(channel),
                endpoint_override: Mutex::new(None),
                current_sequence: Mutex::new(current_sequence_clone),
                current_version: Mutex::new(current_version_clone),
                pending_manifest: Mutex::new(None),
                live_asset_dir,
            });
            Ok(())
        })
        .build();

    Ok((plugin, context))
}

fn resolve_base_dir(app_id: &str) -> PathBuf {
    #[cfg(not(target_os = "android"))]
    {
        let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
        base.join(app_id).join("hotswap")
    }
    #[cfg(target_os = "android")]
    {
        PathBuf::from("/data/data")
            .join(app_id)
            .join("files/hotswap")
    }
}
