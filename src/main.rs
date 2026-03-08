#![no_std]
#![no_main]

//! Custom Controller 固件入口（Embassy async）。
//!
//! 这份工程按“硬件约束先固化，再逐步落地驱动/协议”的方式推进。
//! 所有引脚/外设事实以工程根目录的 `HARDWARE.md` 为准。
//!
//! ## 模块分层（从底到上）
//! - `board`：板级/SoC 级配置（主要是 RCC/时钟）。
//! - `uart`：串口驱动层。
//!   - 4 路位置传感器：DMA + IDLE 分帧接收。
//!   - 上位机链路：USART1 (TX-only) 发送。
//! - `adc`：ADC1 三通道（摇杆）采样任务，维护“最新值快照”。
//! - `input`：按键周期扫描 + 20ms 软件去抖，维护“稳定态快照”。
//! - `protocol`：协议/编码（当前包含“临时最小上报帧”编码器）。
//! - `app`：应用层聚合与联调闭环（轮询输入快照 → 编码 → 上报）。
//!
//! ## 并发模型（给 C++ 背景的快速对照）
//! - `#[embassy_executor::task]` 可以理解为“协程任务/轻量线程”。
//! - 共享的“最新值快照”用 `embassy_sync::Mutex` 保护；
//!   这相当于在中断安全的临界区里做短临界读写（避免 data race）。
//! - 本工程偏向“上层轮询快照”，暂不做事件回调/推送队列。

//! ## Reading Guide（建议阅读顺序）
//! 1. `main()`：看启动顺序与各模块如何 `init/spawn`。
//! 2. `app::host_report_task`：看“轮询输入 → 编码 → 发送”的联调闭环。
//! 3. `protocol::telemetry`：看上报帧的字节布局与端序。
//! 4. `input::buttons`：看按键扫描与 20ms 去抖状态机。
//! 5. `adc::joystick`：看 ADC1 三通道 DMA ring-buffer 的采样方式。
//! 6. `uart::*`：看 4 路 DMA+IDLE 分帧接收与上位机 TX-only 发送。

//! ## Bring-up Checklist（从 0 到 1 联调清单）
//! - 构建：运行 `cargo build`，确认能通过。
//! - 烧录运行：运行 `cargo run`（runner 在 `.cargo/config.toml`，默认使用 probe-rs）。
//! - 日志：通过 RTT/defmt 观察到启动日志 `custom-controller boot` 以及每秒一次的 `tick`。
//! - 输入采样：摇杆与按键任务会在后台运行；你可以先不接协议，直接关注“是否稳定运行”。
//! - 上位机链路（USART1 TX-only）：固件会每 20ms 发送一帧最小上报帧。
//!   - 帧格式见 `HARDWARE.md -> 4.2.1 临时最小上报帧`
//!   - 如需抓包：请在板上 RS485 物理层之后，用合适的适配器/接收端读取 bytes 并按文档解码

use defmt_rtt as _;
use panic_probe as _;

use embassy_executor::Spawner;
use embassy_stm32::Config;

mod adc;
mod app;
mod board;
mod input;
mod protocol;
mod uart;

#[embassy_executor::main]
/// 固件主入口。
///
/// 启动顺序（高层概览）：
/// 1. 配置 RCC/时钟（见 `board::apply_clock_config`）
/// 2. 初始化并启动 UART：
///    - 4 路传感器 RX 任务（DMA + IDLE）
///    - 上位机 TX-only 句柄（USART1）
/// 3. 初始化并启动 ADC1 摇杆采样任务
/// 4. 初始化并启动按键扫描 + 20ms 去抖任务
/// 5. 启动联调上报任务：周期轮询输入并发送最小上报帧
async fn main(spawner: Spawner) {
    // 1) 先配置 RCC/时钟，再初始化外设。
    let mut config = Config::default();
    board::apply_clock_config(&mut config);

    let p = embassy_stm32::init(config);

    defmt::info!("custom-controller boot");

    // 2) 串口初始化：
    // - 4 路传感器 RX 任务会在 `uart::init` 内部 spawn
    // - 上位机 TX-only 句柄会在 `uart::host_tx` 内部缓存
    uart::init(
        spawner,
        p.UART4,
        p.PA1,
        p.DMA1_CH2,
        p.USART2,
        p.PA3,
        p.DMA1_CH5,
        p.USART3,
        p.PC11,
        p.DMA1_CH1,
        p.USART6,
        p.PC7,
        p.DMA2_CH1,
        p.USART1,
        p.PA9,
        p.DMA2_CH7,
    )
    .await;

    // 3) ADC 初始化：spawn 摇杆采样任务。
    adc::init(spawner, p.ADC1, p.DMA2_CH0, p.PB0, p.PA4, p.PA5);

	// 4) Buttons 初始化：spawn 去抖扫描任务。
	input::init(spawner, p.PC0, p.PC1, p.PC2, p.PC3, p.PB7);

    // 5) 联调闭环：周期轮询输入快照并上报到上位机。
    spawner.spawn(app::host_report_task()).unwrap();

    // M1/M2：先保证能启动、能跑 executor。
    // M3~M5：后续在这里按模块 spawn 各任务（UART/ADC/Buttons）。
    loop {
        embassy_time::Timer::after_millis(1000).await;
        defmt::info!("tick");
    }
}
