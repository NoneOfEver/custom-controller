# custom-controller

- 硬件规格：见 HARDWARE.md
- 工程计划：见 ENGINEERING_PLAN.md

本工程目标：把一个 STM32F446 + Embassy 的真实外设驱动工程，整理成“Rust 初学者也能读懂/改得动”的教程级骨架。

## 工程导览（先看这里）

- 入口与启动顺序：`src/main.rs`
- 板级时钟/RCC：`src/board.rs`
- 串口（4 路 DMA+IDLE RX + 1 路 USART1 TX-only）：`src/uart/*`
- ADC1 摇杆三轴采样（DMA ring buffer）：`src/adc/*`
- 按键扫描 + 20ms 去抖：`src/input/*`
- 协议层（当前有“临时最小上报帧”编码器）：`src/protocol/*`
- 应用层聚合与联调闭环（轮询输入→编码→上报）：`src/app/mod.rs`

如果你来自 C/C++：建议按上述顺序读，每个模块顶部都有 `//!` 风格的“模块文档注释”。

## Rust/Embassy 初学者提示（和 C++ 对照）

- `#![no_std]` / `#![no_main]`：嵌入式固件通常不使用标准库（没有 OS/文件/堆等），入口由 `cortex-m-rt`/Embassy 提供。
- 所有权（Ownership）：外设句柄（如 `peripherals::USART1`、DMA 通道、引脚）会被“移动(move)”进 `init(...)` 或任务函数；移动后原变量不能再用，这相当于“编译期保证不会被重复初始化/重复使用”。
- `async/await`：Embassy 把“等待外设/DMA/定时器”的过程做成 `await`，避免阻塞整个系统；可以把每个 `#[embassy_executor::task]` 理解成“协程任务”。
- `Mutex`：本工程用 `embassy_sync::Mutex` 存储“最新值快照”（按键/摇杆/上位机 TX 句柄），避免并发读写数据竞争。
- 无堆分配：DMA buffer 用 `static_cell::StaticCell` 放在静态内存；不依赖 heap。

## 联调：上位机最小上报帧（12 bytes）

固件会每 20ms 发送一帧，格式在 HARDWARE.md 的“4.2.1 临时最小上报帧”里。

你可以用类似下面的 Python 伪代码做解码（示意）：

```python
def parse_frame(frame: bytes):
	assert len(frame) == 12
	assert frame[0] == 0xA5 and frame[1] == 0x5A
	ver = frame[2]
	seq = frame[3]
	axis_a = int.from_bytes(frame[4:6], 'little')
	axis_b = int.from_bytes(frame[6:8], 'little')
	axis_c = int.from_bytes(frame[8:10], 'little')
	buttons = frame[10]
	checksum = frame[11]
	x = 0
	for b in frame[2:11]:
		x ^= b
	assert x == checksum
	return ver, seq, axis_a, axis_b, axis_c, buttons
```

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
