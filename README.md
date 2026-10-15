# Tryingopen-2api（fork 版）QwenPaw 部署

基于 [lza6/Tryingopen-2api](https://github.com/lza6/Tryingopen-2api) v0.1.17 的定制 fork。

## 本 fork 的改动（相对上游）

1. **真实 POST 预检**：`free_proxy.rs` 的预检从 `GET generate_204` 改为真实 `POST /api/open`
   （取首个 SSE chunk 即断开），只选真能打通 tryingopen 的代理。
2. **预检只做一次**：已在池中的代理不再重复预检（省配额），靠 `mark_failure` 自然淘汰。
3. **出口 IP 感知**：预检时经代理查 `api.ipify.org` 学到出口 IP；某代理 429 后该出口 IP
   熔断 24h，重试自动换不同出口 IP。
4. **`allow_local_proxies`**（默认 false）：允许 127.0.0.1/内网地址进池子，
   用于本地 mihomo 一节点一端口的架构（一端口 = 一独立出口 IP）。
5. **`node_sources`**（实验性）：`ss://` 链接或订阅 URL，逗号分隔；网关内嵌 SS 桥
   转为本地代理。已验证可用，但推荐用 mihomo 方案（全协议）。

## 快速开始

```bash
git clone https://github.com/Cornelius798/tryingopen-2api.git
cd tryingopen-2api
chmod +x tryingopen2api
./tryingopen2api --config config.json
```

## 推荐架构（mihomo 节点池）

```
公开订阅/自有订阅
  → mihomo（provider 加载，全协议：ss/trojan/vless/vmess/hy2）
  → url-test 健康检查
  → 一节点一本地端口（127.0.0.1:20011、20012…，各独立出口 IP）
  → proxy_file（每行一个本地端口）
  → 网关（allow_local_proxies=true）
  → 按出口 IP 配额调度（20/天/IP）+ 429 熔断
```

## 关键配置项（config.json）

| 配置 | 默认 | 说明 |
|---|---|---|
| `allow_local_proxies` | `false` | 设 `true` 放行 127.0.0.1/内网代理（mihomo 本地端口必需） |
| `direct_fallback` | `true` | 直连兜底；机房 IP 被 Vercel 挡时设 `false` |
| `free_proxy_enabled` | `true` | 43 个免费源自动抓取；用 mihomo 池时可关 |
| `proxy_file` | `""` | 自备代理文件，每行一个 URL（支持 127.0.0.1，需开 allow_local_proxies） |
| `node_sources` | `""` | 实验性 SS 订阅直连（逗号分隔 ss:// 或订阅 URL） |
| `node_base_port` | `19000` | node_sources 的本地桥起始端口 |
| `hourly_per_ip` | `20` | tryingopen 单 IP 24h 配额（站点硬限制） |
| `max_attempts` | `3` | 单请求最大代理尝试数 |

环境变量同样可用（大写）：`ALLOW_LOCAL_PROXIES=1`、`DIRECT_FALLBACK=false` 等。

## 容量估算

- 容量 = **唯一出口 IP 数** × 20/天（不是代理条目数；很多节点共用出口 IP）
- 公共免费节点大多是机房 IP，可能被 Vercel 风控挑出来，需实测

## 说明

- 二进制：Linux x86-64，v0.1.17（fork）
- 监听：`listen` 在 config.json，默认 `127.0.0.1:47831`
- 上游更新检查 cron 已停用；fork 手动维护
