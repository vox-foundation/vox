//! Length-prefixed frames: `u32` header length, JSON header, `u64` body length, body.
//!
//! A declared body longer than the fetch cap is rejected before the body is read.

use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};

use crate::BridgeError;

const MAX_HEADER_LEN: u32 = 16 * 1024 * 1024;

pub(crate) async fn read_frame<R>(
    reader: &mut R,
    max_body: u64,
) -> Result<Option<(Vec<u8>, Option<Vec<u8>>)>, BridgeError>
where
    R: AsyncRead + Unpin,
{
    let mut len_buf = [0u8; 4];
    let read = reader.read(&mut len_buf).await?;
    if read == 0 {
        return Ok(None);
    }
    if read < 4 {
        reader.read_exact(&mut len_buf[read..]).await?;
    }
    let header_len = u32::from_le_bytes(len_buf);
    if header_len > MAX_HEADER_LEN {
        return Err(BridgeError::protocol("py: header exceeds cap"));
    }
    let mut header = vec![0u8; header_len as usize];
    if header_len > 0 {
        reader.read_exact(&mut header).await?;
    }
    let mut body_len_buf = [0u8; 8];
    reader.read_exact(&mut body_len_buf).await?;
    let body_len = u64::from_le_bytes(body_len_buf);
    trace_frame(header_len, body_len);
    if body_len > max_body {
        return Err(BridgeError::protocol("py: fetch exceeds cap"));
    }
    if body_len == 0 {
        return Ok(Some((header, None)));
    }
    let body_len =
        usize::try_from(body_len).map_err(|_| BridgeError::protocol("py: fetch exceeds cap"))?;
    let mut body = vec![0u8; body_len];
    reader.read_exact(&mut body).await?;
    Ok(Some((header, Some(body))))
}

pub(crate) async fn write_frame<W>(
    writer: &mut W,
    header: &[u8],
    body: Option<&[u8]>,
) -> Result<(), BridgeError>
where
    W: AsyncWrite + Unpin,
{
    let header_len =
        u32::try_from(header.len()).map_err(|_| BridgeError::protocol("py: header exceeds cap"))?;
    if header_len > MAX_HEADER_LEN {
        return Err(BridgeError::protocol("py: header exceeds cap"));
    }
    writer.write_all(&header_len.to_le_bytes()).await?;
    writer.write_all(header).await?;
    let body_bytes = body.unwrap_or_default();
    let body_len = u64::try_from(body_bytes.len())
        .map_err(|_| BridgeError::protocol("py: fetch exceeds cap"))?;
    writer.write_all(&body_len.to_le_bytes()).await?;
    if !body_bytes.is_empty() {
        writer.write_all(body_bytes).await?;
    }
    writer.flush().await?;
    Ok(())
}

fn trace_frame(header_len: u32, body_len: u64) {
    if std::env::var_os("VOX_BRIDGE_FRAME_DEBUG").is_some() {
        eprintln!("vox-python-bridge frame header_len={header_len} body_len={body_len}");
    }
}
