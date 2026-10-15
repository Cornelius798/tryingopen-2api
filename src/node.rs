//! 节点解析：ss:// 链接与订阅 URL
//!
//! Phase 1: Shadowsocks
//! Phase 2: Trojan (planned)
//! Phase 3: VLESS/VMess (planned)

use base64::{engine::general_purpose::STANDARD as B64, Engine};

/// 解析后的节点配置（目前仅 SS）
#[derive(Debug, Clone)]
pub struct NodeConfig {
    pub protocol: NodeProtocol,
    pub server: String,
    pub port: u16,
    /// SS: 加密方法；Trojan: 固定 "trojan"
    pub method: String,
    /// SS: 密码；Trojan/VLESS: password/uuid
    pub password: String,
    /// 显示名（URL fragment）
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub enum NodeProtocol {
    Shadowsocks,
    // Trojan, Vless, Vmess (future phases)
}

/// 解析单个节点 URL（目前仅 ss://）
pub fn parse_node_url(url: &str) -> Option<NodeConfig> {
    let url = url.trim();
    if url.starts_with("ss://") {
        parse_ss_url(url)
    } else {
        None
    }
}

/// 解析 ss:// 链接，支持三种常见格式：
/// 1. ss://method:password@host:port#name
/// 2. ss://base64(method:password@host:port)#name
/// 3. ss://base64(method:password)@host:port#name
fn parse_ss_url(url: &str) -> Option<NodeConfig> {
    let rest = url.strip_prefix("ss://")?;
    // 分离 fragment（名称）
    let (main, name) = match rest.find('#') {
        Some(i) => (&rest[..i], url_decode(&rest[i + 1..])),
        None => (rest, String::new()),
    };

    // 尝试格式1：明文 method:password@host:port
    if let Some(cfg) = parse_ss_plain(main, &name) {
        return Some(cfg);
    }
    // 尝试格式2：整体 base64
    if let Some(decoded) = b64_decode_safe(main) {
        if let Some(cfg) = parse_ss_plain(&decoded, &name) {
            return Some(cfg);
        }
    }
    // 尝试格式3：base64(method:password)@host:port
    if let Some(at) = main.find('@') {
        let (b64part, hostport) = (&main[..at], &main[at + 1..]);
        if let Some(decoded) = b64_decode_safe(b64part) {
            let combined = format!("{decoded}@{hostport}");
            if let Some(cfg) = parse_ss_plain(&combined, &name) {
                return Some(cfg);
            }
        }
    }
    None
}

fn parse_ss_plain(main: &str, name: &str) -> Option<NodeConfig> {
    let at = main.rfind('@')?;
    let (userinfo, hostport) = (&main[..at], &main[at + 1..]);
    let colon = userinfo.find(':')?;
    let (method, password) = (&userinfo[..colon], &userinfo[colon + 1..]);

    // host:port，支持 IPv6 [::1]:port
    let (server, port) = if let Some(stripped) = hostport.strip_prefix('[') {
        let end = stripped.find("]:")?;
        (stripped[..end].to_string(), stripped[end + 2..].parse::<u16>().ok()?)
    } else {
        let colon = hostport.rfind(':')?;
        (hostport[..colon].to_string(), hostport[colon + 1..].parse::<u16>().ok()?)
    };

    if method.is_empty() || password.is_empty() || server.is_empty() {
        return None;
    }
    Some(NodeConfig {
        protocol: NodeProtocol::Shadowsocks,
        server,
        port,
        method: method.to_string(),
        password: url_decode(password),
        name: name.to_string(),
    })
}

/// 解析订阅内容：base64 编码的多行节点链接，或明文多行
pub fn parse_subscription(content: &str) -> Vec<NodeConfig> {
    // 先尝试整体 base64 解码
    let text = b64_decode_safe(content.trim()).unwrap_or_else(|| content.to_string());
    text.lines()
        .filter_map(|line| parse_node_url(line.trim()))
        .collect()
}

/// 抓取订阅 URL 并解析（返回节点列表）
pub async fn fetch_subscription(url: &str) -> Vec<NodeConfig> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        .build();
    let Ok(client) = client else { return vec![] };
    let Ok(resp) = client.get(url).send().await else { return vec![] };
    let Ok(text) = resp.text().await else { return vec![] };
    parse_subscription(&text)
}

fn b64_decode_safe(s: &str) -> Option<String> {
    // 兼容无 padding 的 base64
    let mut padded = s.trim().to_string();
    while padded.len() % 4 != 0 {
        padded.push('=');
    }
    use base64::engine::general_purpose::URL_SAFE;
    // 先试 STANDARD，再试 URL_SAFE
    if let Ok(bytes) = B64.decode(&padded) {
        if let Ok(text) = String::from_utf8(bytes) {
            return Some(text);
        }
    }
    if let Ok(bytes) = URL_SAFE.decode(&padded) {
        if let Ok(text) = String::from_utf8(bytes) {
            return Some(text);
        }
    }
    None
}

fn url_decode(s: &str) -> String {
    // 简单的 percent-decode
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let h1 = chars.next().unwrap_or('0');
            let h2 = chars.next().unwrap_or('0');
            if let Ok(byte) = u8::from_str_radix(&format!("{h1}{h2}"), 16) {
                out.push(byte as char);
                continue;
            }
            out.push('%');
            out.push(h1);
            out.push(h2);
        } else if c == '+' {
            out.push(' ');
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_ss_plain_format() {
        let cfg = parse_node_url("ss://aes-256-gcm:mypassword@1.2.3.4:8388#test-node").unwrap();
        assert_eq!(cfg.method, "aes-256-gcm");
        assert_eq!(cfg.password, "mypassword");
        assert_eq!(cfg.server, "1.2.3.4");
        assert_eq!(cfg.port, 8388);
        assert_eq!(cfg.name, "test-node");
    }

    #[test]
    fn parse_ss_base64_full() {
        // base64("aes-256-gcm:pass@5.6.7.8:9000")
        let cfg = parse_node_url("ss://YWVzLTI1Ni1nY206cGFzc0A1LjYuNy44OjkwMDA=#n2").unwrap();
        assert_eq!(cfg.server, "5.6.7.8");
        assert_eq!(cfg.port, 9000);
    }

    #[test]
    fn parse_subscription_lines() {
        let sub = "ss://aes-256-gcm:p1@1.1.1.1:1111#a\nss://aes-128-gcm:p2@2.2.2.2:2222#b";
        let nodes = parse_subscription(sub);
        assert_eq!(nodes.len(), 2);
    }
}
