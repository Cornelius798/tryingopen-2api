//! 节点桥：把 SS 节点转为本地 HTTP CONNECT 代理，供池子使用
//!
//! 每个节点绑定 127.0.0.1:PORT，池子看到的就是普通 HTTP 代理。

use std::net::SocketAddr;
use std::str::FromStr;
use std::sync::Arc;

use shadowsocks::config::{ServerAddr, ServerConfig, ServerType};
use shadowsocks::context::Context;
use shadowsocks::crypto::CipherKind;
use shadowsocks::relay::socks5::Address;
use shadowsocks::relay::tcprelay::ProxyClientStream;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};

use crate::node::NodeConfig;

/// 常用 SS 加密方法字符串 → CipherKind（仅 AEAD，覆盖绝大多数节点）
fn parse_cipher(s: &str) -> Option<CipherKind> {
    Some(match s.to_lowercase().as_str() {
        "aes-256-gcm" => CipherKind::AES_256_GCM,
        "aes-128-gcm" => CipherKind::AES_128_GCM,
        "chacha20-poly1305" | "chacha20-ietf-poly1305" => CipherKind::CHACHA20_POLY1305,
        _ => return None,
    })
}

/// 启动一个 SS 节点的本地桥，返回本地代理 URL（如 http://127.0.0.1:19001）
pub async fn start_ss_bridge(cfg: &NodeConfig, local_port: u16) -> anyhow::Result<String> {
    let cipher = parse_cipher(&cfg.method)
        .ok_or_else(|| anyhow::anyhow!("不支持的 SS 加密方法: {}", cfg.method))?;
    let server_addr: ServerAddr = format!("{}:{}", cfg.server, cfg.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("SS 服务器地址解析失败: {e}"))?;
    let server_cfg = ServerConfig::new(server_addr, cfg.password.clone(), cipher)
        .map_err(|e| anyhow::anyhow!("SS 配置错误 {}: {e}", cfg.name))?;

    let context = Context::new_shared(ServerType::Local);
    let server_cfg = Arc::new(server_cfg);

    let addr: SocketAddr = format!("127.0.0.1:{local_port}").parse()?;
    let listener = TcpListener::bind(addr).await?;

    tracing::info!(
        "节点桥启动: {} -> 127.0.0.1:{local_port} ({}:{})",
        cfg.name,
        cfg.server,
        cfg.port
    );

    tokio::spawn(async move {
        loop {
            let Ok((inbound, _)) = listener.accept().await else {
                continue;
            };
            let ctx = context.clone();
            let scfg = server_cfg.clone();
            tokio::spawn(async move {
                if let Err(e) = handle_connect(inbound, ctx, scfg).await {
                    tracing::debug!("节点桥连接错误: {e}");
                }
            });
        }
    });

    Ok(format!("http://127.0.0.1:{local_port}"))
}

/// 处理一个 HTTP CONNECT 请求，经 SS 转发
async fn handle_connect(
    inbound: TcpStream,
    context: Arc<Context>,
    server_cfg: Arc<ServerConfig>,
) -> anyhow::Result<()> {
    let mut reader = BufReader::new(inbound);
    let mut request_line = String::new();
    reader.read_line(&mut request_line).await?;

    // 解析 "CONNECT host:port HTTP/1.1"
    let parts: Vec<&str> = request_line.trim().split_whitespace().collect();
    if parts.len() < 2 || parts[0].to_uppercase() != "CONNECT" {
        return Err(anyhow::anyhow!("非 CONNECT 请求"));
    }
    let target_str = parts[1].to_string();

    // 读完剩余 headers（到空行）
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
    }

    // 目标地址解析为 shadowsocks Address
    let target = Address::from_str(&target_str)
        .map_err(|e| anyhow::anyhow!("目标地址解析失败 {target_str}: {e}"))?;

    // 经 SS 连接目标
    let outbound = ProxyClientStream::connect(context, &server_cfg, target)
        .await
        .map_err(|e| anyhow::anyhow!("SS 连接 {target_str} 失败: {e}"))?;

    // 回 200
    let mut inbound = reader.into_inner();
    inbound
        .write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n")
        .await?;

    // 双向转发
    let (mut ri, mut wi) = tokio::io::split(inbound);
    let (mut ro, mut wo) = tokio::io::split(outbound);
    let _ = tokio::join!(
        tokio::io::copy(&mut ri, &mut wo),
        tokio::io::copy(&mut ro, &mut wi),
    );
    Ok(())
}

/// 批量启动节点桥，返回本地代理 URL 列表
pub async fn start_bridges(nodes: &[NodeConfig], base_port: u16) -> Vec<String> {
    let mut urls = Vec::new();
    for (i, node) in nodes.iter().enumerate() {
        let port = base_port + i as u16;
        match start_ss_bridge(node, port).await {
            Ok(url) => urls.push(url),
            Err(e) => tracing::warn!("节点桥启动失败 {}: {e}", node.name),
        }
    }
    urls
}
