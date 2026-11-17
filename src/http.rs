//! The built-in HTTP client and its TLS setup.
//!
//! The TLS backend is chosen with Cargo features (see
//! `docs/decisions/0001-network-layer-and-tls.md`):
//!
//! - `rustls-tls` (default): rustls. Desktop and iOS verify certificates with
//!   the OS; Android uses the bundled webpki roots.
//! - `native-tls` / `native-tls-vendored`: the OS TLS stack. Wins if both are
//!   enabled, because enabling it on top of the defaults is an explicit choice.

use std::sync::OnceLock;

use crate::error::{Error, Result};

#[cfg(not(any(feature = "rustls-tls", feature = "native-tls")))]
compile_error!(
    "tauri-plugin-hotswap needs a TLS backend: enable the `rustls-tls` (default), \
     `native-tls` or `native-tls-vendored` feature."
);

// With both TLS features enabled native-tls wins, so the rustls-only
// dependencies are compiled but unused; mark them as intentionally so.
#[cfg(all(feature = "rustls-tls", feature = "native-tls"))]
use rustls as _;
#[cfg(all(
    feature = "rustls-tls",
    feature = "native-tls",
    not(target_os = "android")
))]
use rustls_platform_verifier as _;
#[cfg(all(feature = "rustls-tls", feature = "native-tls", target_os = "android"))]
use webpki_roots as _;

static DEFAULT_CLIENT: OnceLock<reqwest::Client> = OnceLock::new();

/// The plugin's default HTTP client, built once and shared (connection pool
/// included) by the resolvers and bundle downloads.
pub(crate) fn default_client() -> Result<reqwest::Client> {
    if let Some(client) = DEFAULT_CLIENT.get() {
        return Ok(client.clone());
    }
    let client = build_client()?;
    // If another thread won the race, use its client; both are equivalent.
    Ok(DEFAULT_CLIENT.get_or_init(|| client).clone())
}

fn build_client() -> Result<reqwest::Client> {
    let builder = reqwest::Client::builder();

    #[cfg(feature = "native-tls")]
    let builder = builder.tls_backend_native();

    #[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
    let builder = builder.tls_backend_preconfigured(rustls_config()?);

    builder
        .build()
        .map_err(|e| Error::Config(format!("failed to build the HTTP client: {e}")))
}

/// rustls config built explicitly, so the plugin never installs a
/// process-wide crypto provider. If the app installed one, it is used;
/// otherwise `ring`.
#[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
fn rustls_config() -> Result<rustls::ClientConfig> {
    use std::sync::Arc;

    let provider = rustls::crypto::CryptoProvider::get_default()
        .cloned()
        .unwrap_or_else(|| Arc::new(rustls::crypto::ring::default_provider()));
    let tls_error = |e: rustls::Error| Error::Config(format!("TLS setup failed: {e}"));

    let builder = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(tls_error)?;

    #[cfg(not(target_os = "android"))]
    let builder = {
        use rustls_platform_verifier::BuilderVerifierExt;
        builder.with_platform_verifier().map_err(tls_error)?
    };

    #[cfg(target_os = "android")]
    let builder = {
        let roots = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };
        builder.with_root_certificates(roots)
    };

    Ok(builder.with_no_client_auth())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_client_builds_and_is_shared() {
        // Fails with "Unknown TLS backend" if our rustls version drifts from
        // the one reqwest uses.
        default_client().expect("default client builds");
        default_client().expect("second call reuses the client");
    }

    #[cfg(all(feature = "rustls-tls", not(feature = "native-tls")))]
    #[test]
    fn rustls_config_does_not_install_a_global_provider() {
        let had_default = rustls::crypto::CryptoProvider::get_default().is_some();
        rustls_config().expect("rustls config builds");
        assert_eq!(
            rustls::crypto::CryptoProvider::get_default().is_some(),
            had_default
        );
    }
}
