# 本地构建（Debian 12 / glibc 2.36）

预编译二进制是在 Ubuntu 24.04（glibc 2.39）上编的，Debian 12 跑不起来。
在目标机器上直接编译：

```bash
# 安装 Rust（需 >= 1.87）
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env

# 编译
cargo build --release

# 二进制在 target/release/tryingopen2api
./target/release/tryingopen2api --config config.json
```

首次编译约 3-5 分钟（依赖多）。
