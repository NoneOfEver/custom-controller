# custom-controller

- 硬件规格：见 HARDWARE.md
- 工程计划：见 ENGINEERING_PLAN.md

## 环境准备

- 安装 Rust（建议使用 rustup）
- 安装 probe-rs（用于烧录/运行）：`cargo install probe-rs-tools`
- 确保已安装目标：`rustup target add thumbv7em-none-eabihf`

## 构建

- `cargo build`

## 烧录/运行（SWD）

- `cargo run`

说明：runner 在 .cargo/config.toml 里使用 `probe-rs run --chip STM32F446RETx`。
如果你使用的 probe 或芯片字符串不同，请在该文件里改 runner。
