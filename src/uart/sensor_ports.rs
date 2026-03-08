//! 4 路位置传感器 UART 接收骨架（DMA + IDLE）。
//!
//! 约束来自 HARDWARE.md：
//! - UART4  (PA0/PA1)
//! - USART2 (PA2/PA3)
//! - USART3 (PC10/PC11)
//! - USART6 (PC6/PC7)
//! - TTL, 9600 8N1
//! - 接收：DMA + IDLE 分帧
//!
//! 协议解析由你后续补充：这里仅提供“帧 bytes 到上层”的接口预留。

use embassy_stm32::mode::Async;
use embassy_stm32::usart::UartRx;

#[derive(Copy, Clone, Eq, PartialEq, defmt::Format)]
pub enum SensorPortId {
    Uart4,
    Usart2,
    Usart3,
    Usart6,
}

/// 协议层接口占位：收到一帧 bytes 时调用。
///
/// 后续你可以把它改为 trait + 实现，或改成 channel/queue 推送。
pub fn on_sensor_frame(_port: SensorPortId, _frame: &[u8]) {
    // 占位：协议由你后补
}

#[embassy_executor::task]
pub async fn sensor_rx_task(mut rx: UartRx<'static, Async>, port: SensorPortId) {
    // 这里的 buffer 是“分帧最大长度”的上限。
    // 如果后续协议帧更长，可以再调大。
    let mut buf = [0u8; 128];

    loop {
        match rx.read_until_idle(&mut buf).await {
            Ok(0) => {}
            Ok(n) => on_sensor_frame(port, &buf[..n]),
            Err(e) => {
                defmt::warn!("{:?} rx error: {:?}", port, e);
            }
        }
    }
}
