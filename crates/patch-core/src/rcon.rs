use crate::error::{Error, Result};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

const AUTH_TYPE: i32 = 3;
const COMMAND_TYPE: i32 = 2;
const RESPONSE_TYPE: i32 = 0;

fn encode_packet(id: i32, packet_type: i32, body: &str) -> Vec<u8> {
    let body_bytes = body.as_bytes();
    let size = 4 + 4 + body_bytes.len() + 2;
    let mut buf = Vec::with_capacity(4 + size);
    buf.extend_from_slice(&(size as i32).to_le_bytes());
    buf.extend_from_slice(&id.to_le_bytes());
    buf.extend_from_slice(&packet_type.to_le_bytes());
    buf.extend_from_slice(body_bytes);
    buf.push(0);
    buf.push(0);
    buf
}

async fn read_one_packet(stream: &mut TcpStream, buffer: &mut Vec<u8>) -> Result<(i32, i32, String)> {
    while buffer.len() < 4 {
        let n = stream.read_buf(buffer).await?;
        if n == 0 {
            return Err(Error::Other("RCON connection closed".into()));
        }
    }
    let size = i32::from_le_bytes(buffer[0..4].try_into().unwrap()) as usize;
    if size < 10 || size > 4096 {
        return Err(Error::Other(format!("invalid RCON packet size {size}")));
    }
    let total = 4 + size;
    while buffer.len() < total {
        let n = stream.read_buf(buffer).await?;
        if n == 0 {
            return Err(Error::Other("RCON connection closed mid-packet".into()));
        }
    }
    let id = i32::from_le_bytes(buffer[4..8].try_into().unwrap());
    let ty = i32::from_le_bytes(buffer[8..12].try_into().unwrap());
    let body_end = total - 2;
    let body = std::str::from_utf8(&buffer[12..body_end])
        .map_err(|e| Error::Other(format!("RCON body utf8: {e}")))?
        .to_string();
    buffer.drain(0..total);
    Ok((id, ty, body))
}

pub async fn send_rcon_command(host: &str, port: u16, password: &str, command: &str) -> Result<String> {
    let addr = format!("{host}:{port}");
    let connect = timeout(Duration::from_secs(5), TcpStream::connect(&addr)).await;
    let mut stream = connect
        .map_err(|_| Error::Other("RCON connect timed out".into()))?
        .map_err(|e| Error::Other(format!("RCON connect failed: {e}")))?;

    let mut buffer = Vec::new();
    let auth = encode_packet(1, AUTH_TYPE, password);
    stream.write_all(&auth).await?;

    let (auth_id, _, _) = read_one_packet(&mut stream, &mut buffer).await?;
    if auth_id == -1 {
        return Err(Error::Other("RCON authentication failed".into()));
    }

    let cmd_packet = encode_packet(2, COMMAND_TYPE, command);
    stream.write_all(&cmd_packet).await?;

    let mut output = String::new();
    while let Ok((id, ty, body)) = read_one_packet(&mut stream, &mut buffer).await {
        if id != 2 || ty != RESPONSE_TYPE {
            break;
        }
        output.push_str(&body);
        if body.is_empty() {
            break;
        }
    }
    Ok(output.trim().to_string())
}
