//! Capacitor mobile primitive emit: `@back_button`, `@deep_link`, `@push` → `mobile.ts`.

use vox_compiler::hir::nodes::HirModule;

/// Emit a `mobile.ts` file containing Capacitor setup helpers for any mobile
/// primitives declared in the module.  Returns `None` when no mobile
/// primitives are present so the caller can skip creating the file.
pub fn emit_mobile_setup(hir: &HirModule) -> Option<String> {
    let mut parts: Vec<String> = vec![];

    // ── D2: @back_button ──────────────────────────────────────────────────
    if let Some(back) = &hir.back_button {
        let on_press = &back.on_press;
        let fallback_call = back
            .fallback
            .as_ref()
            .map(|f| format!("await endpoints.{f}();"))
            .unwrap_or_else(|| "App.exitApp();".into());
        parts.push(format!(
            "import {{ App }} from '@capacitor/app';
import * as endpoints from './vox-client';
let __backHandlerRegistered = false;
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

    // ── D3: @deep_link ────────────────────────────────────────────────────
    if let Some(dl) = &hir.deep_link {
        let on_link = &dl.on_link;
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
        let reg_call = if on_reg.is_empty() {
            String::new()
        } else {
            format!(
                "    const token = await PushNotifications.getDeliveredNotifications();\n    await endpoints.{on_reg}(token.notifications[0]?.id ?? '');\n"
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
            "import {{ PushNotifications }} from '@capacitor/push-notifications';
export async function installPushNotifications() {{
  const result = await PushNotifications.requestPermissions();
  if (result.receive === 'granted') {{
    await PushNotifications.register();
  }}
{reg_call}{notif_listener}{action_listener}}}"
        ));
    }

    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n\n"))
    }
}
