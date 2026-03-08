//! 上位机链路：USART1 → RS485（仅发送）。
//!
//! 约束来自 HARDWARE.md：
//! - USART1: TX=PA9, RX=PA10（软件不启用 RX）
//! - 仅发送，不接收，不需要方向控制。

//! ## 使用方式
//! - 初始化阶段调用 [`init`]，把 USART1 TX-only 句柄写入全局缓存。
//! - 运行时任何任务都可以调用 [`send_to_host`] 发送 bytes。
//!
//! ## 并发与性能
//! - 内部用 `Mutex<Option<UartTx>>` 存放句柄：
//!   - 未初始化时发送会打印 `warn` 并丢弃
//!   - 初始化后发送会在 mutex 内 `await` 写出（串行化发送）
//! - 这是“最简单可用”的实现；如果未来上报频率提高，建议换成 channel + 单发送任务。

//! ## Beginner Notes
//! - `Option<T>`：表达“可能没有值”。这里表示 TX 句柄可能尚未初始化。
//! - `Mutex<..., Option<UartTx>>`：把共享资源包进互斥量，避免并发任务同时访问。
//! - `await`：`send_to_host` 会等待 DMA 发送完成；期间当前任务让出执行权，不会阻塞其它任务。

//! ## Reading Guide
//! - 只需关注两个入口：[`init`] 与 [`send_to_host`]。
//! - 初始化顺序由 `uart::init(...).await` 保证（先 init，后发送）。

/// 协议层接口占位：把 bytes 发给上位机。
///
/// 后续实现时可以改为：
/// - 背景任务 + Channel 队列
/// - 或直接在调用点 await write
use embassy_stm32::mode::Async;
use embassy_stm32::usart::{self, UartTx};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;

static HOST_TX: Mutex<CriticalSectionRawMutex, Option<UartTx<'static, Async>>> = Mutex::new(None);

// Rust 初学者提示：
// - `static` 表示全局静态变量（整个程序生命周期存在）。
// - 这里用 `Option<UartTx<...>>` 表达“可能尚未初始化”。
// - 用 `Mutex` 保护它，避免多个 async 任务同时写 USART 造成竞态。

/// 初始化上位机 TX-only 串口。
///
/// 该函数会创建 `UartTx` 并写入全局静态变量，供 [`send_to_host`] 使用。
///
/// # 注意
/// - 本工程硬件约束为“仅发送”；因此这里不创建 RX。
/// - `config` 通常与传感器串口共享（同为 `9600 8N1`）。
pub async fn init(
    usart1: embassy_stm32::peripherals::USART1,
    tx: embassy_stm32::peripherals::PA9,
    tx_dma: embassy_stm32::peripherals::DMA2_CH7,
    config: usart::Config,
) {
    let tx = usart::UartTx::new(usart1, tx, tx_dma, config).unwrap();
    *HOST_TX.lock().await = Some(tx);
}

/// 协议层接口：把 bytes 发给上位机（USART1 TX-only）。
///
/// - 若 [`init`] 尚未调用：记录一次 `defmt::warn!` 并返回。
/// - 若已初始化：异步写出（使用 DMA）。
#[allow(dead_code)]
pub async fn send_to_host(bytes: &[u8]) {
    let mut guard = HOST_TX.lock().await;
    let Some(tx) = guard.as_mut() else {
        defmt::warn!("host_tx not initialized");
        return;
    };

    if let Err(e) = tx.write(bytes).await {
        defmt::warn!("host_tx write error: {:?}", e);
    }
}
