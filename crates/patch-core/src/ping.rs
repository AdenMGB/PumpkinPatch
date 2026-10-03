use crate::error::Result;
use crate::models::PingResult;
use std::time::{Duration, Instant};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;
use tokio::time::timeout;

/// Minimal Minecraft server list ping (protocol 47+ style handshake + status request).
pub async fn ping_server(host: &str, port: u16) -> Result<PingResult> {
    let addr = format!("{host}:{port}");
    let start = Instant::now();
    let connect = timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await;
    let mut stream = match connect {
        Ok(Ok(s)) => s,
        _ => {
            return Ok(PingResult {
                online: false,
                latency_ms: None,
                version: None,
                motd: None,
                players_online: None,
                players_max: None,
            });
        }
    };

    let mut handshake = Vec::new();
    write_varint(&mut handshake, 0x00);
    write_varint(&mut handshake, 47);
    write_string(&mut handshake, host);
    write_unsigned_short(&mut handshake, port);
    write_varint(&mut handshake, 1);
    write_packet(&mut stream, &handshake).await?;

    let mut status_req = Vec::new();
    write_varint(&mut status_req, 0x00);
    write_packet(&mut stream, &status_req).await?;

    let payload = read_packet(&mut stream).await?;
    let json = parse_status_json(&payload)?;
    let latency_ms = start.elapsed().as_millis() as u64;

    Ok(PingResult {
        online: true,
        latency_ms: Some(latency_ms),
        version: json
            .get("version")
            .and_then(|v| v.get("name"))
            .and_then(|n| n.as_str())
            .map(str::to_string),
        motd: json
            .get("description")
            .map(|d| d.to_string())
            .or_else(|| json.get("description").and_then(|d| d.as_str()).map(str::to_string)),
        players_online: json
            .get("players")
            .and_then(|p| p.get("online"))
            .and_then(|n| n.as_u64())
            .map(|n| n as u32),
        players_max: json
            .get("players")
            .and_then(|p| p.get("max"))
            .and_then(|n| n.as_u64())
            .map(|n| n as u32),
    })
}

fn parse_status_json(payload: &[u8]) -> Result<serde_json::Value> {
    if payload.is_empty() {
        return Ok(serde_json::json!({}));
    }
    let mut idx = 0usize;
    let _packet_id = read_varint(payload, &mut idx)?;
    let json = read_string(payload, &mut idx)?;
    Ok(serde_json::from_str(&json)?)
}

async fn write_packet(stream: &mut TcpStream, data: &[u8]) -> std::io::Result<()> {
    let mut frame = Vec::new();
    write_varint(&mut frame, data.len() as i32);
    frame.extend_from_slice(data);
    stream.write_all(&frame).await?;
    Ok(())
}

async fn read_packet(stream: &mut TcpStream) -> std::io::Result<Vec<u8>> {
    let len = read_varint_stream(stream).await? as usize;
    let mut buf = vec![0u8; len];
    stream.read_exact(&mut buf).await?;
    Ok(buf)
}

async fn read_varint_stream(stream: &mut TcpStream) -> std::io::Result<i32> {
    let mut num_read = 0;
    let mut result = 0i32;
    loop {
        let mut buf = [0u8; 1];
        stream.read_exact(&mut buf).await?;
        let value = buf[0];
        result |= ((value & 0x7F) as i32) << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "varint too big",
            ));
        }
        if (value & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn write_varint(buf: &mut Vec<u8>, mut value: i32) {
    loop {
        let mut temp = (value & 0x7F) as u8;
        value >>= 7;
        if value != 0 {
            temp |= 0x80;
        }
        buf.push(temp);
        if value == 0 {
            break;
        }
    }
}

fn read_varint(buf: &[u8], idx: &mut usize) -> std::io::Result<i32> {
    let mut num_read = 0;
    let mut result = 0i32;
    loop {
        if *idx >= buf.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::UnexpectedEof,
                "eof",
            ));
        }
        let value = buf[*idx];
        *idx += 1;
        result |= ((value & 0x7F) as i32) << (7 * num_read);
        num_read += 1;
        if num_read > 5 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "varint too big",
            ));
        }
        if (value & 0x80) == 0 {
            break;
        }
    }
    Ok(result)
}

fn write_string(buf: &mut Vec<u8>, s: &str) {
    write_varint(buf, s.len() as i32);
    buf.extend_from_slice(s.as_bytes());
}

fn read_string(buf: &[u8], idx: &mut usize) -> std::io::Result<String> {
    let len = read_varint(buf, idx)? as usize;
    if *idx + len > buf.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::UnexpectedEof,
            "string eof",
        ));
    }
    let s = String::from_utf8_lossy(&buf[*idx..*idx + len]).into_owned();
    *idx += len;
    Ok(s)
}

fn write_unsigned_short(buf: &mut Vec<u8>, value: u16) {
    buf.push((value >> 8) as u8);
    buf.push((value & 0xFF) as u8);
}
