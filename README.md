# Tryingopen-2api QwenPaw 部署

## 快速开始

```bash
git clone https://github.com/Cornelius798/tryingopen-2api.git
cd tryingopen-2api
chmod +x tryingopen2api
./tryingopen2api --config config.json
```

## 代理来源说明

网关有两个代理来源，**主次关系**如下：

1. **主：网关自带 43 个免费源**（`free_proxy_enabled=true`）
   - 每 30 分钟自动抓取、预检、注入
   - 在 QwenPaw（阿里云）网络下实测可用，这是主力

2. **备：GitHub 代理池**（`data/proxies.txt`）
   - 手动更新：`curl -s https://raw.githubusercontent.com/Cornelius798/proxy-pool/main/pool.txt -o data/proxies.txt`
   - 重启网关生效
   - 作为补充，不是主力

## 说明

- 二进制：Linux x86-64，v0.1.17
- 监听：`listen_addr` 在 config.json 里，默认 `127.0.0.1:47831`，需外部访问改 `0.0.0.0:47831`
