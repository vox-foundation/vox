//! Capacitor mobile primitive emit: `@back_button`, `@deep_link`, `@push` → `mobile.ts`.

use vox_compiler::hir::nodes::HirModule;

/// Emit a `mobile.ts` file containing Capacitor setup helpers for any mobile
/// primitives declared in the module.  Returns `None` when no mobile
/// primitives are present so the caller can skip creating the file.
pub fn emit_mobile_setup(hir: &HirModule) -> Option<String> {
    let has_back = hir.back_button.is_some();
    let has_deep = hir.deep_link.is_some();
    let has_push = hir.push.is_some();

    if !has_back && !has_deep && !has_push {
        return None;
    }

    // Collect imports once; deduplicated by construction.
    let mut imports: Vec<&'static str> = vec!["import * as endpoints from './vox-client';"];
    if has_back || has_deep {
        imports.push("import { App } from '@capacitor/app';");
    }
    if has_deep {
        imports.push("import { useEffect } from 'react';");
        imports.push("import { useNavigate } from '@tanstack/react-router';");
    }
    if has_push {
        imports.push("import { PushNotifications } from '@capacitor/push-notifications';");
    }

    let mut parts: Vec<String> = vec![imports.join("\n")];

    // ── D2: @back_button ──────────────────────────────────────────────────
    if let Some(back) = &hir.back_button {
        let on_press = &back.on_press;
        if on_press.is_empty() {
            // Defensive: skip if required field missing (parser silent-ignore case).
        } else {
            let fallback_call = back
                .fallback
                .as_ref()
                .map(|f| format!("await endpoints.{f}();"))
                .unwrap_or_else(|| "App.exitApp();".into());
            parts.push(format!(
                "let __backHandlerRegistered = false;
export function installBackButtonHandler() {{
  if (__backHandlerRegistered) return;
  __backHandlerRegistered = true;
  App.addListener('backButton', async () => {{
    const handled = await endpoints.{on_press}();
    if (!handled) {{
      {fallback_call}
    }}
  }});
}}"
            ));
        }
    }

    // ── D3: @deep_link ────────────────────────────────────────────────────
    if let Some(dl) = &hir.deep_link {
        let on_link = &dl.on_link;
        // Note: `scheme` and `universal_link` are parsed/stored for future URL-scheme
        // validation; currently the on_link handler is responsible for scheme checks.
        parts.push(format!(
            "export function useDeepLinkRouting() {{
  const navigate = useNavigate();
  useEffect(() => {{
    const sub = App.addListener('appUrlOpen', async (data: {{ url: string }}) => {{
      const target = await endpoints.{on_link}(data.url);
      navigate({{ to: target }});
    }});
    return () => {{ sub.then((s: {{ remove(): void }}) => s.remove()); }};
  }}, [navigate]);
}}"
        ));
    }

    // ── D4: @push ─────────────────────────────────────────────────────────
    if let Some(push) = &hir.push {
        let on_reg = push.on_register.as_deref().unwrap_or("");
        let on_notif = push.on_notification.as_deref().unwrap_or("");
        let on_action = push.on_action.as_deref().unwrap_or("");

        // Listeners must be registered BEFORE calling PushNotifications.register()
        // so the `registration` event is not missed.
        let reg_listener = if on_reg.is_empty() {
            String::new()
        } else {
            format!(
                "  PushNotifications.addListener('registration', async (token) => {{ await endpoints.{on_reg}(token.value); }});\n"
            )
        };
        let notif_listener = if on_notif.is_empty() {
            String::new()
        } else {
            format!(
                "  PushNotifications.addListener('pushNotificationReceived', async (n) => {{ await endpoints.{on_notif}(JSON.stringify(n)); }});\n"
            )
        };
        let action_listener = if on_action.is_empty() {
            String::new()
        } else {
            format!(
                "  PushNotifications.addListener('pushNotificationActionPerformed', async (a) => {{ await endpoints.{on_action}(JSON.stringify(a)); }});\n"
            )
        };
        parts.push(format!(
            "export async function installPushNotifications() {{
{reg_listener}{notif_listener}{action_listener}  const result = await PushNotifications.requestPermissions();
  if (result.receive === 'granted') {{
    await PushNotifications.register();
  }}
}}"
        ));
    }

    Some(parts.join("\n\n"))
}
