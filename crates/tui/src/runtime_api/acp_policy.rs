//! Who may open an ACP connection over the network.
//!
//! The `/acp` endpoint is not another metadata route: a connection to it can
//! read files and run shell commands. It also arrives as a WebSocket upgrade,
//! and upgrades are shaped in two ways that defeat the checks guarding
//! `/v1/*`:
//!
//! 1. **CORS does not apply.** A browser sends no preflight for
//!    `new WebSocket(...)`, so the `CorsLayer` on the router never sees it.
//! 2. **An upgrade is a `GET`.** [`super::auth::web_cookie_request_is_same_origin`]
//!    admits a `GET` that carries no `Origin` at all, which is correct for the
//!    read-only routes it was written for and wrong here.
//!
//! Together those meant any page open in the user's browser could connect to
//! `ws://127.0.0.1:7878/acp`, have the browser attach the session cookie on its
//! own, and obtain a full ACP channel — cross-site WebSocket hijacking that
//! lands as local code execution. The upstream transport crate does not close
//! this either: its `CorsOptions::Disabled` also treats a missing `Origin` as
//! permitted. So the policy lives here, and it fails closed.
//!
//! The rule that makes this tractable: **a browser cannot set request headers
//! on `new WebSocket(...)`**. A valid `Authorization: Bearer` is therefore
//! evidence the caller is not a page, which is what lets non-browser clients
//! (an editor, `websocat`, our own tests) connect with no `Origin` while a
//! cookie-only request with no `Origin` is refused.

use axum::http::{HeaderMap, header};

/// Why an ACP connection was refused. Callers map these to a status.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AcpDenial {
    /// No credential at all, or one that did not match.
    Unauthorized,
    /// Authenticated, but the browser origin is not allowed to reach `/acp`.
    ForbiddenOrigin,
}

/// The decision, expressed over facts rather than over a request, so the table
/// below is testable without standing up a server.
///
/// * `origin` — the `Origin` header, if the caller sent one.
/// * `has_header_credential` — a valid `Authorization: Bearer` or
///   `X-*-Runtime-Token`; proof the caller is not a browser page.
/// * `has_cookie_credential` — a valid runtime or web-session cookie.
/// * `allowed_origins` — the server's own origin plus anything the operator
///   added with `--acp-allow-origin`.
pub(super) fn decide(
    origin: Option<&str>,
    has_header_credential: bool,
    has_cookie_credential: bool,
    allowed_origins: &[String],
) -> Result<(), AcpDenial> {
    if !has_header_credential && !has_cookie_credential {
        return Err(AcpDenial::Unauthorized);
    }

    match origin {
        // An origin the operator did not allow is refused even with a valid
        // bearer token: defence in depth costs nothing here, and a request
        // carrying both a token and a foreign origin is not a shape we want to
        // serve.
        Some(origin) if !allowed_origins.iter().any(|allowed| allowed == origin) => {
            Err(AcpDenial::ForbiddenOrigin)
        }
        Some(_) => Ok(()),
        // No `Origin`. Only a header credential can vouch for this, because a
        // page cannot produce one.
        None if has_header_credential => Ok(()),
        None => Err(AcpDenial::ForbiddenOrigin),
    }
}

/// The origin the server serves itself on, which is always allowed.
#[must_use]
pub(super) fn self_origin(bind_host: &str, bind_port: u16) -> String {
    if bind_port == 80 {
        format!("http://{bind_host}")
    } else {
        format!("http://{bind_host}:{bind_port}")
    }
}

/// Read the `Origin` header, if present and valid UTF-8.
#[must_use]
pub(super) fn origin_of(headers: &HeaderMap) -> Option<&str> {
    headers
        .get(header::ORIGIN)
        .and_then(|value| value.to_str().ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn allowed() -> Vec<String> {
        vec!["http://127.0.0.1:7878".to_string()]
    }

    #[test]
    fn no_credential_is_unauthorized() {
        assert_eq!(
            decide(None, false, false, &allowed()),
            Err(AcpDenial::Unauthorized)
        );
        assert_eq!(
            decide(Some("http://127.0.0.1:7878"), false, false, &allowed()),
            Err(AcpDenial::Unauthorized)
        );
    }

    /// The regression this module exists for. A page on any site can open a
    /// WebSocket to loopback; the browser attaches the cookie by itself and
    /// sends no `Origin` the old `GET` path would have checked. Refuse it.
    #[test]
    fn cookie_without_an_origin_is_refused() {
        assert_eq!(
            decide(None, false, true, &allowed()),
            Err(AcpDenial::ForbiddenOrigin)
        );
    }

    /// A bearer token cannot be set by `new WebSocket(...)`, so it is proof the
    /// caller is an editor or a script rather than a page.
    #[test]
    fn header_credential_without_an_origin_is_allowed() {
        assert!(decide(None, true, false, &allowed()).is_ok());
    }

    #[test]
    fn allowed_origin_passes_with_either_credential() {
        assert!(decide(Some("http://127.0.0.1:7878"), false, true, &allowed()).is_ok());
        assert!(decide(Some("http://127.0.0.1:7878"), true, false, &allowed()).is_ok());
    }

    #[test]
    fn foreign_origin_is_refused_even_with_a_bearer_token() {
        assert_eq!(
            decide(Some("https://evil.example"), true, true, &allowed()),
            Err(AcpDenial::ForbiddenOrigin)
        );
    }

    /// `null` is what a sandboxed iframe or a `file://` page sends. goose
    /// allows it for its Electron shell; we have no Electron, so it is only
    /// reachable when an operator names it explicitly.
    #[test]
    fn opaque_origin_needs_an_explicit_opt_in() {
        assert_eq!(
            decide(Some("null"), true, false, &allowed()),
            Err(AcpDenial::ForbiddenOrigin)
        );

        let mut with_null = allowed();
        with_null.push("null".to_string());
        assert!(decide(Some("null"), true, false, &with_null).is_ok());
    }

    #[test]
    fn self_origin_omits_the_default_http_port() {
        assert_eq!(self_origin("127.0.0.1", 80), "http://127.0.0.1");
        assert_eq!(self_origin("127.0.0.1", 7878), "http://127.0.0.1:7878");
    }
}
