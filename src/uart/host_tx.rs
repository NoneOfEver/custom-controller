//! 上位机链路：USART1 → RS485（仅发送）。
//!
//! 约束来自 HARDWARE.md：
//! - USART1: TX=PA9, RX=PA10（软件不启用 RX）
//! - 仅发送，不接收，不需要方向控制。

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
