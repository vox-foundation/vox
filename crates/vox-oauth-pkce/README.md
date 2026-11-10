# vox-oauth-pkce

RFC 8252 loopback-server OAuth 2.0 PKCE flow implementation from the [Vox](https://github.com/vox-foundation/vox) project.

`vox-oauth-pkce` provides an in-app OAuth 2.0 PKCE authorization workflow with a local ephemeral loopback server (Axum) and browser redirection (`open`), used for interactive developer authentication and token provisioning. It depends internally only on `vox-http-client`.

## Usage

Add `vox-oauth-pkce` to your `Cargo.toml`:

```toml
[dependencies]
vox-oauth-pkce = "0.6"
```

Initiate a PKCE authentication flow:

```rust
use vox_oauth_pkce::PkceFlow;
```

## License

Apache-2.0
