//! Test double that speaks the bridge frame and never executes Python.
//!
//! `test.oversize` declares a 2 MiB body and does not write it, so a host that
//! reads the body blocks instead of returning `py: fetch exceeds cap`.

use std::io::{self, Read, Write};

fn main() {
    let stdin = io::stdin();
    let mut input = stdin.lock();
    let stdout = io::stdout();
    let mut output = stdout.lock();
    let mut seen: Vec<String> = Vec::new();
    let mut released: Vec<u64> = Vec::new();

    loop {
        let Some((header, _body)) = read_frame(&mut input).unwrap_or(None) else {
            break;
        };
        let req: serde_json::Value = match serde_json::from_slice(&header) {
            Ok(value) => value,
            Err(err) => {
                eprintln!("fake_worker: header json: {err}");
                break;
            }
        };
        let method = req
            .get("method")
            .and_then(|value| value.as_str())
            .unwrap_or("")
            .to_string();
        let id = req.get("id").cloned().unwrap_or(serde_json::Value::Null);

        if method == "test.oversize" {
            let header = result_header(&id, &serde_json::json!({"method": "test.oversize"}));
            if write_declared_body(&mut output, &header, 2 * 1024 * 1024).is_err() {
                break;
            }
            // Stay alive without producing the body. The next read blocks until
            // the host closes stdin or sends another frame.
            let _ = read_frame(&mut input);
            break;
        }

        if method == "release"
            && let Some(ids) = req
                .pointer("/params/ids")
                .and_then(|value| value.as_array())
        {
            for id_value in ids {
                if let Some(id) = id_value.as_u64() {
                    released.push(id);
                }
            }
        }

        seen.push(method);
        let value = serde_json::json!({
            "method": seen.last().cloned().unwrap_or_default(),
            "seen": seen,
            "released": released,
        });
        if write_frame(&mut output, &result_header(&id, &value), &[]).is_err() {
            break;
        }
    }
}

fn result_header(id: &serde_json::Value, value: &serde_json::Value) -> Vec<u8> {
    serde_json::to_vec(&serde_json::json!({
        "id": id,
        "payload": {"type": "result", "value": value}
    }))
    .expect("result header")
}

fn read_frame(input: &mut impl Read) -> io::Result<Option<(Vec<u8>, Vec<u8>)>> {
    let mut len_buf = [0u8; 4];
    match input.read(&mut len_buf)? {
        0 => return Ok(None),
        4 => {}
        n => input.read_exact(&mut len_buf[n..])?,
    }
    let header_len = u32::from_le_bytes(len_buf) as usize;
    if header_len > 16 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fake worker refuses a huge header",
        ));
    }
    let mut header = vec![0u8; header_len];
    input.read_exact(&mut header)?;
    let mut body_len_buf = [0u8; 8];
    input.read_exact(&mut body_len_buf)?;
    let body_len = u64::from_le_bytes(body_len_buf);
    if body_len > 16 * 1024 * 1024 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "fake worker refuses a huge body",
        ));
    }
    let mut body = vec![0u8; body_len as usize];
    if body_len > 0 {
        input.read_exact(&mut body)?;
    }
    Ok(Some((header, body)))
}

fn write_frame(output: &mut impl Write, header: &[u8], body: &[u8]) -> io::Result<()> {
    write_declared_body(output, header, body.len() as u64)?;
    if !body.is_empty() {
        output.write_all(body)?;
    }
    output.flush()
}

fn write_declared_body(output: &mut impl Write, header: &[u8], body_len: u64) -> io::Result<()> {
    let header_len = u32::try_from(header.len()).expect("header fits in u32");
    output.write_all(&header_len.to_le_bytes())?;
    output.write_all(header)?;
    output.write_all(&body_len.to_le_bytes())?;
    output.flush()
}
