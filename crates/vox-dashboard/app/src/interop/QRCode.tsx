import React from "react";

export interface QRCodeProps {
  svg: string;     // server-generated SVG string from mesh_invite::mint
  size?: number;
}

/**
 * QR code renderer for the Add-a-Node wizard (P4-T2).
 *
 * The server generates the SVG (in `mesh_invite::mint`) because we want the
 * QR's content to stay server-authoritative — generating it client-side would
 * mean the SPA can't prove the URL it's encoding matches the URL the
 * orchestrator just minted.
 *
 * The `dangerouslySetInnerHTML` is safe here: the SVG string is produced by
 * the `qrcode` Rust crate which only emits geometric SVG elements (no scripts).
 */
export function QRCode(props: QRCodeProps): React.ReactElement {
  const size = props.size ?? 180;
  return (
    <div
      role="img"
      aria-label="Mesh invite QR code"
      style={{ width: size, height: size }}
      dangerouslySetInnerHTML={{ __html: props.svg }}
    />
  );
}
