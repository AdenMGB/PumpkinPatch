//! Gourd edge proxy: status, login relay, Velocity MODERN forwarding (plangem.md §1.3).

mod forwarding;
mod router;

pub use router::{
    BackendPoolState, GourdRouter, parse_pool_state, predictive_scale_recommendation,
};

use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::watch;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct GourdConfig {
    pub bind: SocketAddr,
    pub motd: String,
    pub max_players: u16,
    pub version_name: String,
    pub protocol_version: i32,
}

pub struct GourdProxy {
    config: Arc<GourdConfig>,
    router: GourdRouter,
    shutdown: watch::Sender<bool>,
}

impl GourdProxy {
    pub fn new(config: GourdConfig, router: GourdRouter) -> Self {
        let (shutdown, _) = watch::channel(false);
        Self {
            config: Arc::new(config),
            router,
            shutdown,
        }
    }

    pub async fn run(self) -> Result<(), std::io::Error> {
        let listener = TcpListener::bind(self.config.bind).await?;
        let mut shutdown_rx = self.shutdown.subscribe();
        loop {
            tokio::select! {
                _ = shutdown_rx.changed() => break,
                accept = listener.accept() => {
                    let (stream, peer) = accept?;
                    let cfg = self.config.clone();
                    let router = self.router.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, peer, cfg, router).await {
                            eprintln!("gourd client {peer} ended: {e}");
                        }
                    });
                }
            }
        }
        Ok(())
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown.send(true);
    }
}

async fn handle_client(
    mut client: TcpStream,
    peer: SocketAddr,
    config: Arc<GourdConfig>,
    router: GourdRouter,
) -> Result<(), std::io::Error> {
    let handshake = protocol::read_handshake(&mut client).await?;
    if handshake.next_state == 1 {
        protocol::write_status_response(
            &mut client,
            &config.motd,
            config.protocol_version,
            &config.version_name,
            0,
            config.max_players,
        )
        .await?;
        return Ok(());
    }

    let backend_addr = router.resolve_backend(&handshake.server_address).await;
    let mut backend = TcpStream::connect(backend_addr).await?;
    protocol::write_handshake_to_backend(
        &mut backend,
        handshake.protocol_version,
        &handshake.server_address,
        handshake.server_port,
        handshake.next_state,
    )
    .await?;

    if handshake.next_state == 2 {
        let addr = peer.ip().to_string();
        protocol::relay_login_preamble(
            &mut client,
            &mut backend,
            &router.forwarding_secret,
            &addr,
        )
        .await?;
    }

    let (mut cr, mut cw) = client.into_split();
    let (mut br, mut bw) = backend.into_split();
    let c2b = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut cr, &mut bw).await;
    });
    let b2c = tokio::spawn(async move {
        let _ = tokio::io::copy(&mut br, &mut cw).await;
    });
    let _ = tokio::join!(c2b, b2c);
    Ok(())
}

mod protocol {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    pub struct Handshake {
        pub protocol_version: i32,
        pub server_address: String,
        pub server_port: u16,
        pub next_state: u8,
    }

    pub async fn read_handshake(stream: &mut TcpStream) -> Result<Handshake, std::io::Error> {
        let packet_len = read_var_int(stream).await?;
        let mut buf = vec![0u8; packet_len as usize];
        stream.read_exact(&mut buf).await?;
        let mut cursor = &buf[..];
        let packet_id = read_var_int_buf(&mut cursor)?;
        if packet_id != 0x00 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected handshake",
            ));
        }
        let protocol_version = read_var_int_buf(&mut cursor)?;
        let server_address = read_string_buf(&mut cursor)?;
        let server_port = read_u16_buf(&mut cursor)?;
        let next_state = read_u8_buf(&mut cursor)?;
        Ok(Handshake {
            protocol_version,
            server_address,
            server_port,
            next_state,
        })
    }

    pub async fn write_handshake_to_backend(
        stream: &mut TcpStream,
        protocol_version: i32,
        address: &str,
        port: u16,
        next_state: u8,
    ) -> Result<(), std::io::Error> {
        let mut payload = Vec::new();
        write_var_int(&mut payload, 0x00);
        write_var_int(&mut payload, protocol_version);
        write_string(&mut payload, address);
        write_u16(&mut payload, port);
        payload.push(next_state);
        write_packet(stream, &payload).await?;
        Ok(())
    }

    pub async fn write_status_response(
        stream: &mut TcpStream,
        motd: &str,
        protocol: i32,
        version_name: &str,
        online: u32,
        max: u16,
    ) -> Result<(), std::io::Error> {
        let json = format!(
            r#"{{"version":{{"name":"{version_name}","protocol":{protocol}}},"players":{{"max":{max},"online":{online}}},"description":{{"text":"{motd}"}}}}"#,
            version_name = escape_json(version_name),
            motd = escape_json(motd),
            protocol = protocol,
            max = max,
            online = online,
        );
        let mut payload = Vec::new();
        write_var_int(&mut payload, 0x00);
        write_string(&mut payload, &json);
        write_packet(stream, &payload).await?;

        let mut ping = Vec::new();
        write_var_int(&mut ping, 0x01);
        write_var_int(&mut ping, 0);
        write_packet(stream, &ping).await?;
        Ok(())
    }

    /// Reads Login Start from the client, injects Velocity MODERN forwarding, forwards Login Start.
    pub async fn relay_login_preamble(
        client: &mut TcpStream,
        backend: &mut TcpStream,
        forwarding_secret: &str,
        client_address: &str,
    ) -> Result<(), std::io::Error> {
        let len = read_var_int(client).await?;
        let mut login_start = vec![0u8; len as usize];
        client.read_exact(&mut login_start).await?;

        let mut cursor = &login_start[..];
        let packet_id = read_var_int_buf(&mut cursor)?;
        if packet_id != 0x00 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected login start",
            ));
        }
        let username = read_string_buf(&mut cursor)?;
        let uuid = parse_login_uuid(&mut cursor, &username);

        let forward_payload = forwarding::modern_player_info(
            forwarding_secret,
            client_address,
            uuid,
            &username,
        );
        send_login_plugin_request(backend, 0, "velocity:player_info", &forward_payload).await?;
        read_login_plugin_response(backend).await?;

        write_packet(backend, &login_start).await?;
        Ok(())
    }

    fn parse_login_uuid(cursor: &mut &[u8], username: &str) -> Uuid {
        if cursor.is_empty() {
            return offline_uuid(username);
        }
        let has_uuid = cursor[0] != 0;
        *cursor = &cursor[1..];
        if has_uuid && cursor.len() >= 16 {
            let mut bytes = [0u8; 16];
            bytes.copy_from_slice(&cursor[..16]);
            *cursor = &cursor[16..];
            return Uuid::from_bytes(bytes);
        }
        offline_uuid(username)
    }

    fn offline_uuid(username: &str) -> Uuid {
        let digest = md5::compute(format!("OfflinePlayer:{}", username).as_bytes());
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&digest.0);
        bytes[6] = (bytes[6] & 0x0f) | 0x30;
        bytes[8] = (bytes[8] & 0x3f) | 0x80;
        Uuid::from_bytes(bytes)
    }

    async fn send_login_plugin_request(
        backend: &mut TcpStream,
        message_id: i32,
        channel: &str,
        data: &[u8],
    ) -> Result<(), std::io::Error> {
        let mut payload = Vec::new();
        write_var_int(&mut payload, 0x04);
        write_var_int(&mut payload, message_id);
        write_string(&mut payload, channel);
        write_var_int(&mut payload, data.len() as i32);
        payload.extend_from_slice(data);
        write_packet(backend, &payload).await?;
        Ok(())
    }

    async fn read_login_plugin_response(backend: &mut TcpStream) -> Result<(), std::io::Error> {
        let len = read_var_int(backend).await?;
        let mut buf = vec![0u8; len as usize];
        backend.read_exact(&mut buf).await?;
        let mut cursor = &buf[..];
        let packet_id = read_var_int_buf(&mut cursor)?;
        if packet_id != 0x02 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "expected login plugin response",
            ));
        }
        let _message_id = read_var_int_buf(&mut cursor)?;
        if cursor.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "login plugin response truncated",
            ));
        }
        if cursor[0] == 0 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "backend rejected velocity forwarding",
            ));
        }
        Ok(())
    }

    async fn read_var_int(stream: &mut TcpStream) -> Result<i32, std::io::Error> {
        let mut num = 0i32;
        let mut count = 0;
        loop {
            let mut b = [0u8; 1];
            stream.read_exact(&mut b).await?;
            let byte = b[0];
            num |= ((byte & 0x7F) as i32) << (count * 7);
            count += 1;
            if count > 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "varint too long",
                ));
            }
            if byte & 0x80 == 0 {
                break;
            }
        }
        Ok(num)
    }

    fn read_var_int_buf(cursor: &mut &[u8]) -> Result<i32, std::io::Error> {
        let mut num = 0i32;
        let mut count = 0;
        while !cursor.is_empty() {
            let byte = cursor[0];
            *cursor = &cursor[1..];
            num |= ((byte & 0x7F) as i32) << (count * 7);
            count += 1;
            if count > 5 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "varint too long",
                ));
            }
            if byte & 0x80 == 0 {
                break;
            }
        }
        Ok(num)
    }

    fn read_string_buf(cursor: &mut &[u8]) -> Result<String, std::io::Error> {
        let len = read_var_int_buf(cursor)? as usize;
        if cursor.len() < len {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "string truncated",
            ));
        }
        let s = String::from_utf8_lossy(&cursor[..len]).into_owned();
        *cursor = &cursor[len..];
        Ok(s)
    }

    fn read_u16_buf(cursor: &mut &[u8]) -> Result<u16, std::io::Error> {
        if cursor.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "u16 truncated",
            ));
        }
        let v = u16::from_be_bytes([cursor[0], cursor[1]]);
        *cursor = &cursor[2..];
        Ok(v)
    }

    fn read_u8_buf(cursor: &mut &[u8]) -> Result<u8, std::io::Error> {
        if cursor.is_empty() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "u8 truncated",
            ));
        }
        let v = cursor[0];
        *cursor = &cursor[1..];
        Ok(v)
    }

    fn write_var_int(buf: &mut Vec<u8>, value: i32) {
        let mut val = value as u32;
        loop {
            let mut temp = (val & 0b0111_1111) as u8;
            val >>= 7;
            if val != 0 {
                temp |= 0b1000_0000;
            }
            buf.push(temp);
            if val == 0 {
                break;
            }
        }
    }

    fn write_string(buf: &mut Vec<u8>, s: &str) {
        write_var_int(buf, s.len() as i32);
        buf.extend_from_slice(s.as_bytes());
    }

    fn write_u16(buf: &mut Vec<u8>, v: u16) {
        buf.extend_from_slice(&v.to_be_bytes());
    }

    async fn write_packet(stream: &mut TcpStream, payload: &[u8]) -> Result<(), std::io::Error> {
        let mut header = Vec::new();
        write_var_int(&mut header, payload.len() as i32);
        stream.write_all(&header).await?;
        stream.write_all(payload).await?;
        Ok(())
    }

    fn escape_json(s: &str) -> String {
        s.replace('\\', "\\\\").replace('"', "\\\"")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{timeout, Duration};

    #[tokio::test]
    async fn proxy_binds_and_accepts() {
        let backend = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let backend_addr = backend.local_addr().unwrap();
        tokio::spawn(async move {
            if let Ok((mut s, _)) = backend.accept().await {
                let _ = s.shutdown();
            }
        });

        let bind = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let bind_addr = bind.local_addr().unwrap();
        let router = GourdRouter::new("test-secret".into(), backend_addr);
        let proxy = GourdProxy::new(
            GourdConfig {
                bind: bind_addr,
                motd: "Gourd test".into(),
                max_players: 100,
                version_name: "1.21.4".into(),
                protocol_version: 767,
            },
            router,
        );
        tokio::spawn(proxy.run());

        let _ = timeout(Duration::from_secs(2), TcpStream::connect(bind_addr))
            .await
            .unwrap();
    }
}
