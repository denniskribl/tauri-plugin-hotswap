# Migrating from 0.0.x to 1.0

This guide grows with each 1.0 change ([roadmap](https://github.com/denniskribl/tauri-plugin-hotswap/issues/22)).

## TLS backend ([#18](https://github.com/denniskribl/tauri-plugin-hotswap/issues/18))

0.0.x picked the TLS stack per platform: the OS stack on desktop and iOS, rustls on Android. 1.0 uses features that apply everywhere, with `rustls-tls` as the default.

**Default features:** no change needed.
- Android: same as before.
- Desktop and iOS: now use rustls. Certificates are still verified by the OS trust store. Servers that only offer TLS 1.0/1.1 are no longer reachable. Linux builds no longer need OpenSSL.

**To keep the OS TLS stack** (0.0.x behaviour on desktop and iOS):

```toml
tauri-plugin-hotswap = { version = "1", default-features = false, features = ["native-tls"] }
```

On Android, `native-tls` needs OpenSSL: link it yourself, or use `native-tls-vendored` to build it from source.

**`default-features = false`:** add `rustls-tls`, `native-tls` or `native-tls-vendored`. Without one, the build fails with an error naming the features. Keep any other features you use (e.g. `zip`).
