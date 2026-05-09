//! Mobile Capacitor primitive declarations: `@back_button`, `@deep_link`, `@push`.

use crate::ast::span::Span;

/// `@back_button { on_press: handler [fallback: handler] }` —
/// registers a Capacitor `App.addListener('backButton', …)` handler.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct BackButtonDecl {
    /// Endpoint function called on back-press; returns bool (handled?).
    pub on_press: String,
    /// Optional fallback function or action when `on_press` returns false.
    pub fallback: Option<String>,
    /// Source span.
    pub span: Span,
}

/// `@deep_link { scheme: "…" on_link: handler [universal_link: "…"] }` —
/// registers a Capacitor `App.addListener('appUrlOpen', …)` handler.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct DeepLinkDecl {
    /// URL scheme (e.g. `"voxmental"`).
    pub scheme: String,
    /// Optional Apple universal link domain.
    pub universal_link: Option<String>,
    /// Endpoint function called with the opened URL; returns the target route path.
    pub on_link: String,
    /// Source span.
    pub span: Span,
}

/// `@push { [on_register: handler] [on_notification: handler] [on_action: handler] }` —
/// wires Capacitor `PushNotifications` registration + listeners.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PushDecl {
    /// Endpoint called after push registration to store the token.
    pub on_register: Option<String>,
    /// Endpoint called when a notification is received in the foreground.
    pub on_notification: Option<String>,
    /// Endpoint called when the user taps a notification action.
    pub on_action: Option<String>,
    /// Source span.
    pub span: Span,
}
