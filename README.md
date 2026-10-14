# Tryingopen-2api QwenPaw 部署

## 快速开始

```bash
git clone https://github.com/Cornelius798/tryingopen-2api-qwenpaw.git
cd tryingopen-2api-qwenpaw
chmod +x tryingopen2api
./tryingopen2api --config config.json
```

## 更新代理池

```bash
curl -s https://raw.githubusercontent.com/Cornelius798/proxy-pool/main/pool.txt -o data/proxies.txt
# 重启网关生效
```

## 说明

- 二进制：Linux x86-64，v0.1.17
- 监听：`listen_addr` 在 config.json 里，默认 `127.0.0.1:47831`，需外部访问改 `0.0.0.0:47831`
- 代理池：`data/proxies.txt`，每 3 小时自动更新（上游仓库）
- 网关自带 43 个免费代理源（`free_proxy_enabled=true`），会定期自动抓取
