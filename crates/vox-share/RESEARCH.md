---
title: "vox share — Tunnel Backend Research"
status: "current"
---

## Cloudflare Quick Tunnels

Cloudflare Quick Tunnels are free, anonymous (no account required), and expose a randomly assigned `*.trycloudflare.com` subdomain per session. The tunnel is established via the `cloudflared` Go binary (~30 MB, Apache-2.0). The URL is ephemeral — it changes on every run.

Key limitation: Cloudflare enforces a **200 in-flight request cap**, which breaks Server-Sent Events (SSE) and long-polling. `vox share` streams output over SSE, so Cloudflare Quick Tunnels cannot be the primary backend for interactive sessions. Cloudflare is retained as a fallback for users who want a quick, shareable URL for non-streaming content.

Source: https://developers.cloudflare.com/cloudflare-one/connections/connect-networks/do-more-with-tunnels/trycloudflare/

## localhost.run

localhost.run is SSH-based: it requires no client binary beyond stock OpenSSH, which ships on every modern OS. URLs are `*.lhr.life` subdomains and change each session. The free tier is **speed-throttled**, making it unsuitable for media-heavy or high-throughput apps. For low-bandwidth, text-oriented sharing (the common `vox share` case) it is a viable no-install option. SSE is not explicitly blocked.

Source: https://localhost.run/docs/

## Tailscale Funnel

Tailscale Funnel exposes a stable `<hostname>.<tailnet>.ts.net` URL on the public internet — the URL does not change between runs. It requires a Tailscale account (free Personal plan) and the `tailscale` CLI to be installed and authenticated on the user's machine. Only ports **443, 8443, and 10000** are supported. There is no extra binary to ship; `vox share` shells out to the installed `tailscale` CLI.

Source: https://tailscale.com/kb/1223/funnel

## Disqualified Options

- **bore** (ekzhang/bore): TCP-only; no HTTPS subdomain, no SNI termination. Pure Rust but cannot provide a browser-accessible web URL.
- **ngrok**: Required an account for any persistent session since 2023; account-gated and proprietary.
- **zrok**: Account-gated self-hosted relay; not suitable as a turnkey zero-config default.
- **localtunnel**: npm-based; requires a Node.js runtime; historically unreliable due to volunteer-run infrastructure.
- **serveo**: SSH-based like localhost.run but unmaintained and frequently unavailable.
- **playit.gg**: Gaming-focused; does not provide HTTPS subdomains suitable for web apps.

## Future Direction

The Vox-hosted FRP relay (`*.vox.live`) is documented in Phase S10 of the plan and follows the HuggingFace/Gradio model (forked `huggingface/frp`, Apache-2.0), providing a first-party backend with full SSE support and stable URLs without requiring any third-party account.

## Decision Summary

| Provider          | Free | No account | SSE support | Stable URL | Selection       |
|-------------------|------|------------|-------------|------------|-----------------|
| Cloudflare        | Yes  | Yes        | No          | No         | Fallback only   |
| localhost.run     | Yes  | Yes        | Yes         | No         | Default option  |
| Tailscale Funnel  | Yes  | No         | Yes         | Yes        | Opt-in (stable) |
| bore              | Yes  | Yes        | No          | No         | Rejected        |
| ngrok             | No   | No         | Yes         | No         | Rejected        |
| zrok              | No   | No         | Yes         | No         | Rejected        |
| localtunnel       | Yes  | Yes        | Yes         | No         | Rejected        |
| serveo            | Yes  | Yes        | Yes         | No         | Rejected        |
| playit.gg         | Yes  | Yes        | No          | No         | Rejected        |
